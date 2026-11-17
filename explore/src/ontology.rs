use crate::data::normalize;
use quran_ui::overlay::AyahKey;
use quran_ui::MUSHAF;
use serde::Deserialize;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Clone, Copy, PartialEq)]
pub enum Edge {
    Named,
    Related,
    Shared(u16),
}

pub struct Concept {
    pub id: u32,
    pub parent: Option<usize>,
    pub arabic: String,
    pub english: String,
    pub description: String,
    pub wiki: String,
    pub ayat: Vec<u16>,
    pub kids: Vec<usize>,
    pub edges: Vec<(usize, Edge)>,
}

pub struct Ontology {
    pub concepts: Vec<Concept>,
    pub keys: Vec<AyahKey>,
    pub of_ayah: HashMap<u16, Vec<usize>>,
    folded: Vec<Vec<u16>>,
}

#[derive(Deserialize)]
struct Raw {
    t: Vec<(u32, i32, String, String, String, String)>,
    a: Vec<Vec<u16>>,
    l: Vec<(usize, usize, u8)>,
    c: Vec<(usize, usize, u16)>,
}

#[derive(Deserialize)]
pub struct RawText {
    k: Vec<u16>,
    t: Vec<String>,
}

pub fn parse_text(json: &str) -> Option<HashMap<u16, String>> {
    let raw: RawText = serde_json::from_str(json).ok()?;
    Some(raw.k.into_iter().zip(raw.t).collect())
}

pub enum Piece {
    Text(String),
    Link(usize, String),
}

/// Splits a description into text and `{index|label}` links.
pub fn pieces(description: &str) -> Vec<Piece> {
    let mut out = vec![];
    let mut rest = description;
    while let Some(open) = rest.find('{') {
        let Some(close) = rest[open..].find('}').map(|c| open + c) else { break };
        let inner = &rest[open + 1..close];
        let link = inner.split_once('|').and_then(|(i, label)| Some(Piece::Link(i.parse().ok()?, label.to_string())));
        out.push(Piece::Text(rest[..open].to_string()));
        out.push(link.unwrap_or_else(|| Piece::Text(inner.to_string())));
        rest = &rest[close + 1..];
    }
    out.push(Piece::Text(rest.to_string()));
    out
}

impl Ontology {
    pub fn parse(json: &str) -> Option<Ontology> {
        let raw: Raw = serde_json::from_str(json).ok()?;
        let n = raw.t.len();
        let mut concepts: Vec<Concept> = raw
            .t
            .into_iter()
            .zip(raw.a)
            .map(|((id, parent, arabic, english, description, wiki), ayat)| Concept {
                id,
                parent: usize::try_from(parent).ok().filter(|&p| p < n),
                arabic,
                english,
                description,
                wiki,
                ayat,
                kids: vec![],
                edges: vec![],
            })
            .collect();
        for i in 0..n {
            if let Some(p) = concepts[i].parent {
                concepts[p].kids.push(i);
            }
        }
        let size = |c: &Concept| c.ayat.len();
        for i in 0..n {
            let mut kids = std::mem::take(&mut concepts[i].kids);
            kids.sort_by_key(|&k| std::cmp::Reverse(size(&concepts[k])));
            concepts[i].kids = kids;
        }
        for (i, j, kind) in raw.l.into_iter().filter(|&(i, j, _)| i < n && j < n) {
            let edge = if kind == 0 { Edge::Named } else { Edge::Related };
            concepts[i].edges.push((j, edge));
            concepts[j].edges.push((i, edge));
        }
        for (i, j, shared) in raw.c.into_iter().filter(|&(i, j, _)| i < n && j < n) {
            concepts[i].edges.push((j, Edge::Shared(shared)));
            concepts[j].edges.push((i, Edge::Shared(shared)));
        }
        let keys: Vec<AyahKey> = MUSHAF.surahs.iter().flat_map(|s| (1..=s.ayat).map(move |a| (s.num as u8, a))).collect();
        let mut of_ayah: HashMap<u16, Vec<usize>> = HashMap::new();
        for (i, c) in concepts.iter().enumerate() {
            for &a in &c.ayat {
                of_ayah.entry(a).or_default().push(i);
            }
        }
        let mut ontology = Ontology { concepts, keys, of_ayah, folded: vec![] };
        ontology.folded = (0..n).map(|i| ontology.collect(i)).collect();
        Some(ontology)
    }

    fn collect(&self, i: usize) -> Vec<u16> {
        let mut all = self.concepts[i].ayat.clone();
        for &k in &self.concepts[i].kids {
            all.extend(self.collect(k));
        }
        all.sort_unstable();
        all.dedup();
        all
    }

    /// The concept's ayat, with those of every sub-concept when `rollup` is set.
    pub fn ayat(&self, i: usize, rollup: bool) -> &[u16] {
        if rollup { &self.folded[i] } else { &self.concepts[i].ayat }
    }

    pub fn key(&self, a: u16) -> AyahKey {
        self.keys[a as usize]
    }

    pub fn slot(&self, key: AyahKey) -> Option<u16> {
        let surah = MUSHAF.surahs.get((key.0 as usize).checked_sub(1)?)?;
        if key.1 == 0 || key.1 > surah.ayat {
            return None;
        }
        let before: u16 = MUSHAF.surahs[..key.0 as usize - 1].iter().map(|s| s.ayat).sum();
        Some(before + key.1 - 1)
    }

    pub fn by_id(&self, id: u32) -> Option<usize> {
        self.concepts.iter().position(|c| c.id == id)
    }

    pub fn lineage(&self, mut i: usize) -> Vec<usize> {
        let mut out = vec![];
        while let Some(p) = self.concepts[i].parent {
            out.push(p);
            i = p;
        }
        out.reverse();
        out
    }

    pub fn roots(&self) -> Vec<usize> {
        let mut roots: Vec<usize> = (0..self.concepts.len()).filter(|&i| self.concepts[i].parent.is_none()).collect();
        roots.sort_by_key(|&i| std::cmp::Reverse(self.folded[i].len()));
        roots
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<usize> {
        let q = query.trim().to_lowercase();
        let ar = normalize(&q);
        if ar.trim().is_empty() {
            return vec![];
        }
        let mut hits: Vec<(usize, usize)> = self
            .concepts
            .iter()
            .enumerate()
            .filter_map(|(i, c)| {
                let english = c.english.to_lowercase();
                let rank = if english == q || normalize(&c.arabic) == ar {
                    0
                } else if english.starts_with(&q) || normalize(&c.arabic).starts_with(&ar) {
                    1
                } else if english.contains(&q) || (!ar.is_empty() && normalize(&c.arabic).contains(&ar)) {
                    2
                } else {
                    return None;
                };
                Some((rank, i))
            })
            .collect();
        hits.sort_by_key(|&(rank, i)| (rank, std::cmp::Reverse(self.folded[i].len())));
        hits.into_iter().take(limit).map(|(_, i)| i).collect()
    }

    /// Ayat shared by two concepts (direct ayat only).
    pub fn shared(&self, i: usize, j: usize) -> Vec<u16> {
        let other = &self.concepts[j].ayat;
        self.concepts[i].ayat.iter().copied().filter(|a| other.binary_search(a).is_ok()).collect()
    }

    /// The sub-concepts of `i` (excluding `i`) that list ayah `a` directly.
    pub fn via(&self, i: usize, a: u16) -> Vec<usize> {
        let mut out = vec![];
        let mut stack = self.concepts[i].kids.clone();
        while let Some(k) = stack.pop() {
            if self.concepts[k].ayat.binary_search(&a).is_ok() {
                out.push(k);
            }
            stack.extend(self.concepts[k].kids.iter().copied());
        }
        out
    }

    pub fn title(&self, i: usize) -> &str {
        let c = &self.concepts[i];
        if c.arabic.is_empty() { &c.english } else { &c.arabic }
    }
}

#[derive(Clone)]
pub struct SharedOntology(pub Rc<Ontology>);

impl PartialEq for SharedOntology {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

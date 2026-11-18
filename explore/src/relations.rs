use crate::data::Data;
use quran_ui::overlay::AyahKey;
use quran_ui::MUSHAF;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::LazyLock;

pub const STORY: u8 = 0;
pub const WORDING: u8 = 1;
pub const ECHO: u8 = 3;
const LEAD_WORDS: usize = 4;

/// Every ayah in mushaf order; relations.json refers to ayat by their index here.
pub static KEYS: LazyLock<Vec<AyahKey>> =
    LazyLock::new(|| MUSHAF.surahs.iter().flat_map(|s| (1..=s.ayat).map(move |a| (s.num as u8, a))).collect());
static FIRST: LazyLock<Vec<u16>> = LazyLock::new(|| {
    let mut acc = 0;
    MUSHAF.surahs.iter().map(|s| { let first = acc; acc += s.ayat; first }).collect()
});

pub fn index_of(key: AyahKey) -> Option<u16> {
    let surah = MUSHAF.surahs.get((key.0 as usize).checked_sub(1)?)?;
    (key.1 >= 1 && key.1 <= surah.ayat).then(|| FIRST[key.0 as usize - 1] + key.1 - 1)
}

pub fn key_of(g: u16) -> AyahKey {
    KEYS[g as usize]
}

type RawLink = (usize, usize, u8, u8, u8, Vec<usize>, Vec<usize>, Vec<usize>, Vec<(u16, usize, u16, usize)>);

#[derive(Deserialize)]
struct Raw {
    v: (usize, usize),
    o: Vec<String>,
    c: Vec<(String, String)>,
    t: Vec<(u16, u16, u16, u16, String, String, i32)>,
    r: Vec<(usize, Vec<(u16, usize)>)>,
    l: Vec<RawLink>,
    x: Vec<(u16, String)>,
}

pub struct Twin {
    pub sides: [(u16, u16); 2],
    pub flags: [Vec<u8>; 2],
    pub frame: i32,
}

pub struct Refrain {
    pub len: usize,
    pub at: Vec<(u16, usize)>,
}

pub struct Link {
    pub i: usize,
    pub j: usize,
    pub kind: u8,
    pub score: u8,
    pub strong: bool,
    pub twins: Vec<usize>,
    pub families: Vec<usize>,
    pub concepts: Vec<usize>,
    pub words: Vec<(u16, usize, u16, usize)>,
}

pub struct Relations {
    pub version: (usize, usize),
    pub openings: Vec<String>,
    pub concepts: Vec<(String, String)>,
    pub twins: Vec<Twin>,
    pub refrains: Vec<Refrain>,
    pub links: Vec<Link>,
    extra: HashMap<u16, String>,
    pub spans: Vec<(u16, u16)>,
    pub links_of: Vec<Vec<usize>>,
    pub twins_of: Vec<Vec<usize>>,
    pub refrains_of: Vec<Vec<usize>>,
}

/// One row of a twin's comparison ladder, from the point of view of one side. Shared words keep their flag and the
/// other passage's form (which differs for ^ vowels and ~ a leading و/ف).
pub enum Seg {
    Same(Vec<(String, u8, String)>),
    Fork(Vec<String>, Vec<String>),
}

fn overlaps(a: (u16, u16), b: (u16, u16)) -> bool {
    a.0 <= b.1 && b.0 <= a.1
}

impl Relations {
    /// `ranges` are the theme rows (surah, from, to); a file built for other themes is rejected.
    pub fn parse(json: &str, ranges: &[(u8, u16, u16)]) -> Option<Relations> {
        let raw: Raw = serde_json::from_str(json).ok()?;
        let n = ranges.len();
        if raw.o.len() != n {
            return None;
        }
        let spans: Vec<(u16, u16)> = ranges.iter().map(|&(s, a, b)| Some((index_of((s, a))?, index_of((s, b))?))).collect::<Option<_>>()?;
        let total = KEYS.len() as u16;
        let twins: Vec<Twin> = raw
            .t
            .into_iter()
            .filter(|t| t.0 <= t.1 && t.1 < total && t.2 <= t.3 && t.3 < total)
            .map(|(a0, a1, b0, b1, fa, fb, frame)| Twin { sides: [(a0, a1), (b0, b1)], flags: [fa.into_bytes(), fb.into_bytes()], frame })
            .collect();
        let concepts = raw.c;
        let links: Vec<Link> = raw
            .l
            .into_iter()
            .filter(|l| l.0 < n && l.1 < n && l.0 != l.1)
            .map(|(i, j, kind, score, strong, twins_k, families, cs, words)| Link {
                i,
                j,
                kind,
                score,
                strong: strong == 1,
                twins: twins_k.into_iter().filter(|&k| k < twins.len()).collect(),
                families,
                concepts: cs.into_iter().filter(|&c| c < concepts.len()).collect(),
                words: words.into_iter().filter(|w| w.0 < total && w.2 < total).collect(),
            })
            .collect();
        let refrains: Vec<Refrain> =
            raw.r.into_iter().filter(|r| r.1.iter().all(|a| a.0 < total)).map(|(len, at)| Refrain { len, at }).collect();
        let mut links_of = vec![vec![]; n];
        for (k, l) in links.iter().enumerate() {
            links_of[l.i].push(k);
            links_of[l.j].push(k);
        }
        let mut twins_of = vec![vec![]; n];
        let mut refrains_of = vec![vec![]; n];
        for (t, &span) in spans.iter().enumerate() {
            twins_of[t] = (0..twins.len()).filter(|&k| twins[k].sides.iter().any(|&s| overlaps(s, span))).collect();
            refrains_of[t] = (0..refrains.len()).filter(|&k| refrains[k].at.iter().any(|a| overlaps((a.0, a.0), span))).collect();
        }
        Some(Relations {
            version: raw.v,
            openings: raw.o,
            concepts,
            twins,
            refrains,
            links,
            extra: raw.x.into_iter().collect(),
            spans,
            links_of,
            twins_of,
            refrains_of,
        })
    }

    pub fn partner(&self, link: usize, theme: usize) -> usize {
        let l = &self.links[link];
        if l.i == theme { l.j } else { l.i }
    }

    pub fn theme_at(&self, g: u16) -> Option<usize> {
        self.spans.iter().position(|&(lo, hi)| lo <= g && g <= hi)
    }

    /// The themes to open for twin `k` seen from `side`: the pair of the link that carries it, else the themes of the two
    /// sides' first ayat; no partner when both sides lie in one theme.
    pub fn twin_target(&self, k: usize, side: usize) -> Option<(usize, Option<usize>)> {
        let s = self.twins[k].sides;
        if let Some(l) = self.links.iter().find(|l| l.twins.contains(&k)) {
            return Some(if overlaps(s[side], self.spans[l.i]) { (l.i, Some(l.j)) } else { (l.j, Some(l.i)) });
        }
        let t = self.theme_at(s[side].0)?;
        Some((t, self.theme_at(s[1 - side].0).filter(|&p| p != t)))
    }

    /// The look-alikes the pair (t, p) owns, with the side that lies in `t`; p None means both sides start in `t`.
    /// Each look-alike belongs to exactly one pair, seen from either side.
    pub fn twins_for(&self, t: usize, p: Option<usize>) -> Vec<(usize, usize)> {
        self.twins_of[t].iter().filter_map(|&k| (0..2).find(|&s| self.twin_target(k, s) == Some((t, p))).map(|s| (k, s))).collect()
    }

    /// The partners the panel of `t` lists: its links, then the owners of look-alikes no link of `t` carries.
    pub fn listed(&self, t: usize) -> Vec<(Option<usize>, usize)> {
        let mut out: Vec<(Option<usize>, usize)> = self.links_of[t].iter().map(|&l| (Some(l), self.partner(l, t))).collect();
        let mut seen: HashSet<usize> = out.iter().map(|r| r.1).collect();
        seen.insert(t);
        for &k in &self.twins_of[t] {
            for s in 0..2 {
                if let Some((owner, Some(p))) = self.twin_target(k, s) {
                    if owner == t && seen.insert(p) {
                        out.push((None, p));
                    }
                }
            }
        }
        out
    }

    pub fn words(&self, g: u16, data: Option<&Data>) -> Option<Vec<String>> {
        if let Some(text) = self.extra.get(&g) {
            return Some(text.split(' ').map(str::to_string).collect());
        }
        let d = data?;
        let a = *d.slot.get(&key_of(g))?;
        Some(d.words(a).into_iter().map(str::to_string).collect())
    }

    /// The ladder of twin `k` seen from `side`: shared stretches once, forks as two cells.
    pub fn ladder(&self, k: usize, side: usize, data: Option<&Data>) -> Option<Vec<Seg>> {
        let twin = &self.twins[k];
        let read = |s: usize| -> Option<Vec<(String, u8)>> {
            let (lo, hi) = twin.sides[s];
            let mut words: Vec<String> = vec![];
            for g in lo..=hi {
                if let Some(last) = words.last_mut() {
                    last.push_str(" ۝");
                }
                words.extend(self.words(g, data)?);
            }
            (words.len() == twin.flags[s].len()).then(|| words.into_iter().zip(twin.flags[s].iter().copied()).collect())
        };
        let (this, other) = (read(side)?, read(1 - side)?);
        let (mut i, mut j, mut segs) = (0, 0, vec![]);
        while i < this.len() || j < other.len() {
            let matched = |v: &Vec<(String, u8)>, x: usize| x < v.len() && v[x].1 != b'*';
            if matched(&this, i) && matched(&other, j) {
                let mut same = vec![];
                while matched(&this, i) && matched(&other, j) {
                    same.push((this[i].0.clone(), this[i].1, other[j].0.clone()));
                    i += 1;
                    j += 1;
                }
                segs.push(Seg::Same(same));
            } else {
                let (mut a, mut b) = (vec![], vec![]);
                while i < this.len() && this[i].1 == b'*' {
                    a.push(this[i].0.clone());
                    i += 1;
                }
                while j < other.len() && other[j].1 == b'*' {
                    b.push(other[j].0.clone());
                    j += 1;
                }
                if a.is_empty() && b.is_empty() {
                    return None;
                }
                segs.push(Seg::Fork(a, b));
            }
        }
        Some(segs)
    }

    /// The opening words of the ayah after `g`, if it is in the same surah.
    pub fn next_words(&self, g: u16, data: Option<&Data>) -> Option<Vec<String>> {
        let next = g + 1;
        (next < KEYS.len() as u16 && key_of(next).0 == key_of(g).0).then(|| self.words(next, data)).flatten().map(|w| w.into_iter().take(LEAD_WORDS).collect())
    }

    pub fn range_label(&self, lo: u16, hi: u16) -> String {
        let (s, a) = key_of(lo);
        if lo == hi { format!("{s}:{a}") } else { format!("{s}:{a}–{}", key_of(hi).1) }
    }
}

#[derive(Clone)]
pub struct SharedRelations(pub Rc<Relations>);

impl PartialEq for SharedRelations {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

use quran_ui::overlay::AyahKey;
use serde::Deserialize;
use std::collections::HashMap;

const LEAF_WORDS: usize = 3;
const BEFORE_WORDS: usize = 4;
const CHAIN_WORDS: usize = 5;

#[derive(Deserialize)]
struct Raw {
    a: Vec<(u8, u16, String)>,
    f: Vec<(usize, Vec<(usize, usize)>)>,
    v: Vec<(f32, Vec<(usize, String)>)>,
}

pub struct Family {
    pub len: usize,
    pub members: Vec<(usize, usize)>,
}

pub struct Variant {
    pub ratio: f32,
    pub members: Vec<(usize, String)>,
}

pub struct Data {
    pub keys: Vec<AyahKey>,
    pub text: Vec<Vec<String>>,
    norm: Vec<Vec<String>>,
    pub families: Vec<Family>,
    pub variants: Vec<Variant>,
    pub slot: HashMap<AyahKey, usize>,
    pub family_of: Vec<Vec<usize>>,
    pub variant_of: Vec<Vec<usize>>,
}

fn normalize(word: &str) -> String {
    word.chars()
        .filter(|c| !matches!(*c as u32, 0x0610..=0x061A | 0x064B..=0x065F | 0x0670 | 0x06D6..=0x06ED))
        .map(|c| match c {
            'أ' | 'إ' | 'آ' | 'ٱ' => 'ا',
            'ى' => 'ي',
            _ => c,
        })
        .collect()
}

impl Data {
    pub fn parse(json: &str) -> Option<Data> {
        let raw: Raw = serde_json::from_str(json).ok()?;
        let keys: Vec<AyahKey> = raw.a.iter().map(|r| (r.0, r.1)).collect();
        let text: Vec<Vec<String>> = raw.a.iter().map(|r| r.2.split(' ').map(String::from).collect()).collect();
        let norm = text.iter().map(|ws| ws.iter().map(|w| normalize(w)).collect()).collect();
        let slot = keys.iter().enumerate().map(|(i, &k)| (k, i)).collect();
        let families: Vec<Family> = raw.f.into_iter().map(|(len, members)| Family { len, members }).collect();
        let variants: Vec<Variant> = raw.v.into_iter().map(|(ratio, members)| Variant { ratio, members }).collect();
        let mut family_of = vec![vec![]; keys.len()];
        for (f, fam) in families.iter().enumerate() {
            for &(a, _) in &fam.members {
                family_of[a].push(f);
            }
        }
        let mut variant_of = vec![vec![]; keys.len()];
        for (v, var) in variants.iter().enumerate() {
            for (a, _) in &var.members {
                variant_of[*a].push(v);
            }
        }
        Some(Data { keys, text, norm, families, variants, slot, family_of, variant_of })
    }

    pub fn start_in(&self, family: usize, ayah: usize) -> usize {
        self.families[family].members.iter().find(|m| m.0 == ayah).map_or(0, |m| m.1)
    }

    pub fn phrase(&self, family: usize) -> String {
        let (a, start) = self.families[family].members[0];
        self.text[a][start..start + self.families[family].len].join(" ")
    }

    pub fn default_root(&self, ayah: usize) -> Option<usize> {
        self.family_of[ayah].iter().copied().max_by_key(|&f| (self.families[f].members.len(), self.families[f].len))
    }

    pub fn tree(&self, family: usize) -> (Vec<Node>, Vec<Node>) {
        let fam = &self.families[family];
        let after = fam.members.iter().map(|&(a, s)| (a, (s + fam.len..self.text[a].len()).collect::<Vec<_>>())).collect();
        let before = fam.members.iter().map(|&(a, s)| (a, (s.saturating_sub(BEFORE_WORDS)..s).rev().collect::<Vec<_>>())).collect();
        (self.branch(after, false), self.branch(before, true))
    }

    fn branch(&self, items: Vec<(usize, Vec<usize>)>, reversed: bool) -> Vec<Node> {
        let mut groups: Vec<(Option<String>, Vec<(usize, Vec<usize>)>)> = vec![];
        for (a, idx) in items {
            let head = idx.first().map(|&i| self.norm[a][i].clone());
            match groups.iter_mut().find(|g| g.0 == head) {
                Some(g) => g.1.push((a, idx)),
                None => groups.push((head, vec![(a, idx)])),
            }
        }
        groups
            .into_iter()
            .map(|(head, group)| {
                let ayat: Vec<usize> = group.iter().map(|g| g.0).collect();
                if head.is_none() {
                    return Node { words: vec![], more: false, ayat, kids: vec![], end: true };
                }
                let (a0, idx0) = &group[0];
                if group.len() == 1 {
                    let take = idx0.len().min(LEAF_WORDS);
                    let words = self.display(*a0, &idx0[..take], reversed);
                    return Node { words, more: idx0.len() > take, ayat, kids: vec![], end: false };
                }
                let mut common = 0;
                while common < CHAIN_WORDS
                    && group.iter().all(|(a, idx)| idx.len() > common && self.norm[*a][idx[common]] == self.norm[*a0][idx0[common]])
                {
                    common += 1;
                }
                let words = self.display(*a0, &idx0[..common], reversed);
                let rest = group.into_iter().map(|(a, idx)| (a, idx[common..].to_vec())).collect();
                let kids = if reversed && common >= BEFORE_WORDS { vec![] } else { self.branch(rest, reversed) };
                Node { words, more: false, ayat, kids, end: false }
            })
            .collect()
    }

    fn display(&self, a: usize, idx: &[usize], reversed: bool) -> Vec<String> {
        let mut words: Vec<String> = idx.iter().map(|&i| self.text[a][i].clone()).collect();
        if reversed {
            words.reverse();
        }
        words
    }
}

#[derive(Clone, PartialEq)]
pub struct Node {
    pub words: Vec<String>,
    pub more: bool,
    pub ayat: Vec<usize>,
    pub kids: Vec<Node>,
    pub end: bool,
}

impl Node {
    pub fn leaves(&self) -> usize {
        if self.kids.is_empty() { 1 } else { self.kids.iter().map(Node::leaves).sum() }
    }
}

pub fn leaves(nodes: &[Node]) -> usize {
    nodes.iter().map(Node::leaves).sum::<usize>().max(1)
}

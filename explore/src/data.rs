use quran_ui::overlay::AyahKey;
use serde::Deserialize;
use std::collections::HashMap;

const LEAF_WORDS: usize = 3;
const BEFORE_WORDS: usize = 4;
const CHAIN_WORDS: usize = 5;
const LATE_MIN_OVERLAP: usize = 2;
const LATE_LIMIT: usize = 4;

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
    pub pos: Vec<f32>,
    text: Vec<String>,
    pub families: Vec<Family>,
    pub variants: Vec<Variant>,
    pub slot: HashMap<AyahKey, usize>,
    pub family_of: Vec<Vec<usize>>,
    pub variant_of: Vec<Vec<usize>>,
}

pub fn normalize(word: &str) -> String {
    word.chars()
        .filter(|c| !matches!(*c as u32, 0x0610..=0x061A | 0x064B..=0x065F | 0x0670 | 0x06D6..=0x06ED))
        .map(|c| match c {
            'أ' | 'إ' | 'آ' | 'ٱ' => 'ا',
            'ى' => 'ي',
            _ => c,
        })
        .collect()
}

fn strip_prefix(word: &str) -> &str {
    if word.chars().count() > 3 { word.strip_prefix(['و', 'ف']).unwrap_or(word) } else { word }
}

type Word = (String, String);

struct Item {
    ayah: usize,
    late: bool,
    seq: Vec<Word>,
}

#[derive(Clone, PartialEq, Default)]
pub struct Node {
    pub words: Vec<String>,
    pub rest: Vec<String>,
    pub ayat: Vec<usize>,
    pub late: Vec<usize>,
    pub kids: Vec<Node>,
    pub end: bool,
    pub prefix: bool,
    pub forms: Vec<String>,
}

impl Node {
    pub fn leaves(&self) -> usize {
        if self.kids.is_empty() { 1 } else { self.kids.iter().map(Node::leaves).sum() }
    }

    pub fn all_late(&self) -> bool {
        !self.ayat.is_empty() && self.ayat.iter().all(|a| self.late.contains(a))
    }
}

pub fn leaves(nodes: &[Node]) -> usize {
    nodes.iter().map(Node::leaves).sum::<usize>().max(1)
}

const CUE_TAIL: usize = 2;
const CUE_MIN: usize = 3;
const CUE_BEFORE: usize = 2;

#[derive(Clone, PartialEq)]
pub struct Row {
    pub ayah: usize,
    pub before: Vec<String>,
    pub before_cue: Option<usize>,
    pub before_more: bool,
    pub after: Vec<String>,
    pub after_cue: Option<usize>,
    pub after_more: bool,
}

fn diverge(seqs: &[Vec<Word>], i: usize) -> usize {
    seqs.iter()
        .enumerate()
        .filter(|&(j, _)| j != i)
        .map(|(_, other)| seqs[i].iter().zip(other).take_while(|(a, b)| a.0 == b.0).count())
        .max()
        .unwrap_or(0)
}

pub struct TreeData {
    pub after: Vec<Node>,
    pub before: Vec<Node>,
    pub late: Vec<usize>,
}

impl Data {
    pub fn parse(json: &str) -> Option<Data> {
        let raw: Raw = serde_json::from_str(json).ok()?;
        let keys: Vec<AyahKey> = raw.a.iter().map(|r| (r.0, r.1)).collect();
        let text: Vec<String> = raw.a.into_iter().map(|r| r.2).collect();
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
        let pos = keys.iter().map(|&k| quran_ui::overlay::ayah_pos(k)).collect();
        Some(Data { keys, pos, text, families, variants, slot, family_of, variant_of })
    }

    pub fn words(&self, a: usize) -> Vec<&str> {
        self.text[a].split(' ').collect()
    }

    pub fn start_in(&self, family: usize, ayah: usize) -> usize {
        self.families[family].members.iter().find(|m| m.0 == ayah).map_or(0, |m| m.1)
    }

    pub fn span_in(&self, family: usize, ayah: usize) -> (usize, usize) {
        let s = self.start_in(family, ayah);
        (s, s + self.families[family].len)
    }

    pub fn weight(&self, family: usize) -> usize {
        self.families[family].len * self.families[family].members.len()
    }

    pub fn phrase(&self, family: usize) -> String {
        let (a, start) = self.families[family].members[0];
        self.words(a)[start..start + self.families[family].len].join(" ")
    }

    pub fn short_phrase(&self, family: usize, max: usize) -> String {
        let (a, start) = self.families[family].members[0];
        let len = self.families[family].len;
        let words = self.words(a);
        if len <= max + 2 {
            return words[start..start + len].join(" ");
        }
        format!("{} … {}", words[start..start + max - 1].join(" "), words[start + len - 2..start + len].join(" "))
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<usize> {
        let q = normalize(query.trim());
        if q.chars().count() < 2 {
            return vec![];
        }
        let mut hits: Vec<usize> = (0..self.families.len()).filter(|&f| normalize(&self.phrase(f)).contains(&q)).collect();
        hits.sort_by_key(|&f| usize::MAX - self.weight(f));
        hits.truncate(limit);
        hits
    }

    pub fn in_range(&self, lo: f32, hi: f32) -> (Vec<usize>, Vec<usize>) {
        let count = |f: usize| self.families[f].members.iter().filter(|m| self.pos[m.0] >= lo && self.pos[m.0] < hi).count();
        let mut ranked: Vec<(usize, usize)> = (0..self.families.len()).map(|f| (f, count(f))).filter(|&(_, n)| n > 0).collect();
        ranked.sort_by_key(|&(f, _)| usize::MAX - self.weight(f));
        let mut shown: Vec<usize> = vec![];
        let (mut inside, mut elsewhere) = (vec![], vec![]);
        for (f, n) in ranked {
            let members: Vec<usize> = self.families[f].members.iter().map(|m| m.0).collect();
            let covered = shown.iter().any(|&g| members.iter().all(|a| self.families[g].members.iter().any(|m| m.0 == *a)));
            if covered {
                continue;
            }
            shown.push(f);
            if n >= 2 { inside.push(f) } else { elsewhere.push(f) }
        }
        (inside, elsewhere)
    }

    pub fn default_root(&self, ayah: usize) -> Option<usize> {
        self.family_of[ayah].iter().copied().max_by_key(|&f| (self.weight(f), self.families[f].len))
    }

    pub fn folded(&self, family: usize, root: usize, ayah: usize) -> bool {
        let (s, e) = self.span_in(family, ayah);
        let (rs, re) = self.span_in(root, ayah);
        let members = &self.families[root].members;
        family != root && s <= rs && e >= re && self.families[family].members.iter().all(|m| members.iter().any(|r| r.0 == m.0))
    }

    fn seq(&self, a: usize, range: impl Iterator<Item = usize>) -> Vec<Word> {
        let words = self.words(a);
        range.map(|i| (normalize(words[i]), words[i].to_string())).collect()
    }

    fn late_items(&self, root: usize, picked: usize) -> Vec<Item> {
        let (rs, re) = self.span_in(root, picked);
        let in_root: Vec<usize> = self.families[root].members.iter().map(|m| m.0).collect();
        let mut best: HashMap<usize, (usize, usize)> = HashMap::new();
        for &g in &self.family_of[picked] {
            let (gs, ge) = self.span_in(g, picked);
            if !(rs < gs && gs < re && ge >= re && re - gs >= LATE_MIN_OVERLAP) {
                continue;
            }
            let overlap = re - gs;
            for &(a, ms) in &self.families[g].members {
                if in_root.contains(&a) {
                    continue;
                }
                let point = ms + overlap;
                if best.get(&a).is_none_or(|b| b.1 < overlap) {
                    best.insert(a, (point, overlap));
                }
            }
        }
        let mut late: Vec<(usize, usize, usize)> = best.into_iter().map(|(a, (p, o))| (a, p, o)).collect();
        late.sort_by_key(|&(a, _, o)| (usize::MAX - o, a));
        late.truncate(LATE_LIMIT);
        late.sort_by_key(|&(a, ..)| a);
        late.into_iter().map(|(a, p, _)| Item { ayah: a, late: true, seq: self.seq(a, p..self.words(a).len()) }).collect()
    }

    pub fn rows(&self, family: usize) -> Vec<Row> {
        let fam = &self.families[family];
        let after: Vec<Vec<Word>> = fam.members.iter().map(|&(a, s)| self.seq(a, s + fam.len..self.words(a).len())).collect();
        let before: Vec<Vec<Word>> = fam.members.iter().map(|&(a, s)| self.seq(a, (0..s).rev())).collect();
        let mut rows: Vec<Row> = fam
            .members
            .iter()
            .enumerate()
            .map(|(i, &(a, _))| {
                let after_cue = diverge(&after, i);
                let before_cue = diverge(&before, i);
                let after_shown = (after_cue + CUE_TAIL).min(after[i].len()).max(after[i].len().min(CUE_MIN));
                let before_shown = (before_cue + 1).min(before[i].len()).max(before[i].len().min(CUE_BEFORE));
                Row {
                    ayah: a,
                    before: display(&before[i][..before_shown], true),
                    before_cue: (before_cue < before[i].len()).then(|| before_shown - 1 - before_cue),
                    before_more: before_shown < before[i].len(),
                    after: display(&after[i][..after_shown], false),
                    after_cue: (after_cue < after[i].len()).then_some(after_cue),
                    after_more: after_shown < after[i].len(),
                }
            })
            .collect();
        rows.sort_by(|x, y| x.after.first().cmp(&y.after.first()).then(x.ayah.cmp(&y.ayah)));
        rows
    }

    pub fn late_ayat(&self, root: usize, picked: usize) -> Vec<usize> {
        self.late_items(root, picked).into_iter().map(|i| i.ayah).collect()
    }

    pub fn tree(&self, family: usize, picked: usize) -> TreeData {
        let fam = &self.families[family];
        let mut after: Vec<Item> = fam
            .members
            .iter()
            .map(|&(a, s)| Item { ayah: a, late: false, seq: self.seq(a, s + fam.len..self.words(a).len()) })
            .collect();
        let late_items = self.late_items(family, picked);
        let late: Vec<usize> = late_items.iter().map(|i| i.ayah).collect();
        after.extend(late_items);
        let before = fam
            .members
            .iter()
            .map(|&(a, s)| Item { ayah: a, late: false, seq: self.seq(a, (s.saturating_sub(BEFORE_WORDS)..s).rev()) })
            .collect();
        TreeData { after: branch(after, false), before: branch(before, true), late }
    }
}

fn display(words: &[Word], reversed: bool) -> Vec<String> {
    let mut shown: Vec<String> = words.iter().map(|w| w.1.clone()).collect();
    if reversed {
        shown.reverse();
    }
    shown
}

fn branch(items: Vec<Item>, reversed: bool) -> Vec<Node> {
    let mut groups: Vec<(Option<String>, Vec<Item>)> = vec![];
    for item in items {
        let head = item.seq.first().map(|w| w.0.clone());
        match groups.iter_mut().find(|g| g.0 == head) {
            Some(g) => g.1.push(item),
            None => groups.push((head, vec![item])),
        }
    }
    let stems: Vec<Option<String>> = groups.iter().map(|g| g.0.as_deref().map(|w| strip_prefix(w).to_string())).collect();
    groups
        .into_iter()
        .enumerate()
        .map(|(gi, (head, group))| {
            let prefix = stems[gi].is_some() && stems.iter().enumerate().any(|(j, s)| j != gi && *s == stems[gi]);
            let ayat: Vec<usize> = group.iter().map(|i| i.ayah).collect();
            let late: Vec<usize> = group.iter().filter(|i| i.late).map(|i| i.ayah).collect();
            if head.is_none() {
                return Node { end: true, ayat, late, ..Node::default() };
            }
            let first = &group[0].seq;
            let identical = group.iter().all(|i| i.seq.len() == first.len() && i.seq.iter().zip(first).all(|(a, b)| a.0 == b.0));
            if group.len() == 1 || identical {
                let take = first.len().min(LEAF_WORDS);
                let words = display(&first[..take], reversed);
                let rest = if reversed { vec![] } else { display(&first[take..], false) };
                return Node { words, rest, ayat, late, prefix, ..Node::default() };
            }
            let mut common = 0;
            while common < CHAIN_WORDS && group.iter().all(|i| i.seq.len() > common && i.seq[common].0 == first[common].0) {
                common += 1;
            }
            let words = display(&first[..common], reversed);
            let mut forms: Vec<String> = group.iter().map(|i| display(&i.seq[..common], reversed).join(" ")).collect();
            forms.sort();
            forms.dedup();
            let forms = if forms.len() > 1 { forms } else { vec![] };
            let stop = reversed && common >= BEFORE_WORDS;
            let rest: Vec<Item> = group.into_iter().map(|i| Item { seq: i.seq[common..].to_vec(), ..i }).collect();
            let kids = if stop { vec![] } else { branch(rest, reversed) };
            Node { words, ayat, late, kids, prefix, forms, ..Node::default() }
        })
        .collect()
}

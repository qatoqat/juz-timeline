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
        .filter(|c| !matches!(*c as u32, 0x00A0 | 0x0610..=0x061A | 0x064B..=0x065F | 0x0670 | 0x06D6..=0x06ED))
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

/// (matching form, text as written, index in the ayah)
type Word = (String, String, usize);

/// A word as written, without the waqf or section signs attached to it.
fn spelled(word: &str) -> String {
    word.split('\u{a0}').filter(|p| !p.chars().all(|c| matches!(c as u32, 0x06D6..=0x06ED))).collect::<Vec<_>>().join("\u{a0}")
}

/// The distinct spellings, when there is more than one.
fn forms(spellings: impl Iterator<Item = String>) -> Vec<String> {
    let mut forms: Vec<String> = spellings.collect();
    forms.sort();
    forms.dedup();
    if forms.len() > 1 { forms } else { vec![] }
}

/// Words of the picked ayah, by index, that other ayat on its tree path write differently, with each spelling.
pub type Spellings = HashMap<usize, Vec<String>>;

struct Item {
    ayah: usize,
    late: bool,
    seq: Vec<Word>,
    /// Words of the ayah lie beyond `seq` (before-side only: the ayah starts earlier than the window).
    more: bool,
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
    /// A before-side tip that does not reach the start of its ayah (shown with a leading "…").
    pub cut: bool,
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
    pub spellings: Spellings,
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

    /// The family's words as the ayah writes them (empty if the ayah is not a member).
    pub fn span_words(&self, family: usize, ayah: usize) -> Vec<&str> {
        let (s, e) = self.span_in(family, ayah);
        self.words(ayah).get(s..e).map(<[&str]>::to_vec).unwrap_or_default()
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
        range.map(|i| (normalize(words[i]), words[i].to_string(), i)).collect()
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
        late.into_iter().map(|(a, p, _)| Item { ayah: a, late: true, seq: self.seq(a, p..self.words(a).len()), more: false }).collect()
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
            .map(|&(a, s)| Item { ayah: a, late: false, seq: self.seq(a, s + fam.len..self.words(a).len()), more: false })
            .collect();
        let late_items = self.late_items(family, picked);
        let late: Vec<usize> = late_items.iter().map(|i| i.ayah).collect();
        after.extend(late_items);
        let before = fam
            .members
            .iter()
            .map(|&(a, s)| Item { ayah: a, late: false, seq: self.seq(a, (s.saturating_sub(BEFORE_WORDS)..s).rev()), more: s > BEFORE_WORDS })
            .collect();
        let ps = self.start_in(family, picked);
        let mut spellings = Spellings::new();
        for j in 0..fam.len {
            let f = forms(fam.members.iter().map(|&(a, s)| spelled(self.words(a)[s + j])));
            if !f.is_empty() {
                spellings.insert(ps + j, f);
            }
        }
        let after = branch(after, false, picked, &mut spellings);
        let before = branch(before, true, picked, &mut spellings);
        TreeData { after, before, late, spellings }
    }
}

fn display(words: &[Word], reversed: bool) -> Vec<String> {
    let mut shown: Vec<String> = words.iter().map(|w| w.1.clone()).collect();
    if reversed {
        shown.reverse();
    }
    shown
}

/// The node's spellings over its first `n` words, noting each word of the picked ayah written differently.
fn spellings(group: &[Item], n: usize, reversed: bool, picked: usize, out: &mut Spellings) -> Vec<String> {
    if group.len() < 2 {
        return vec![];
    }
    if let Some(me) = group.iter().find(|i| i.ayah == picked) {
        for j in 0..n {
            let f = forms(group.iter().map(|i| spelled(&i.seq[j].1)));
            if !f.is_empty() {
                out.insert(me.seq[j].2, f);
            }
        }
    }
    forms(group.iter().map(|i| {
        let mut words: Vec<String> = i.seq[..n].iter().map(|w| spelled(&w.1)).collect();
        if reversed {
            words.reverse();
        }
        words.join(" ")
    }))
}

fn branch(items: Vec<Item>, reversed: bool, picked: usize, out: &mut Spellings) -> Vec<Node> {
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
            let first = &group[group.iter().position(|i| i.ayah == picked).unwrap_or(0)].seq;
            let identical = group.iter().all(|i| i.seq.len() == first.len() && i.seq.iter().zip(first).all(|(a, b)| a.0 == b.0));
            if group.len() == 1 || identical {
                let take = first.len().min(LEAF_WORDS);
                let words = display(&first[..take], reversed);
                let rest = if reversed { vec![] } else { display(&first[take..], false) };
                let cut = reversed && (first.len() > take || group.iter().any(|i| i.more));
                let forms = spellings(&group, if reversed { take } else { first.len() }, reversed, picked, out);
                return Node { words, rest, ayat, late, prefix, forms, cut, ..Node::default() };
            }
            let mut common = 0;
            while common < CHAIN_WORDS && common < first.len() && group.iter().all(|i| i.seq.len() > common && i.seq[common].0 == first[common].0) {
                common += 1;
            }
            let words = display(&first[..common], reversed);
            let forms = spellings(&group, common, reversed, picked, out);
            let stop = reversed && common >= BEFORE_WORDS;
            let cut = stop && group.iter().any(|i| i.more || i.seq.len() > common);
            let rest: Vec<Item> = group.into_iter().map(|i| Item { seq: i.seq[common..].to_vec(), ..i }).collect();
            let kids = if stop { vec![] } else { branch(rest, reversed, picked, out) };
            Node { words, ayat, late, kids, prefix, forms, cut, ..Node::default() }
        })
        .collect()
}

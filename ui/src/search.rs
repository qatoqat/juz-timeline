use crate::{Focus, MUSHAF};
use dioxus::prelude::*;

const ARTICLES: [&str; 11] = ["al", "aal", "an", "ash", "at", "az", "ad", "ar", "as", "adh", "ath"];

fn normalize(text: &str) -> String {
    let lower = text.to_lowercase();
    let parts: Vec<&str> = lower.split('-').collect();
    let body = if parts.len() > 1 && ARTICLES.contains(&parts[0]) { parts[1..].concat() } else { lower.clone() };
    let mut out = String::new();
    for c in body.chars().filter(|c| c.is_ascii_alphanumeric()) {
        if !out.ends_with(c) {
            out.push(c);
        }
    }
    out.trim_end_matches('h').to_string()
}

fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let cur = row[j + 1];
            row[j + 1] = (prev + (ca != cb) as usize).min(row[j] + 1).min(cur + 1);
            prev = cur;
        }
    }
    row[b.len()]
}

fn surah_score(i: usize, query: &str, number: Option<usize>) -> u32 {
    let s = &MUSHAF.surahs[i];
    if number == Some(s.num) {
        return 5;
    }
    if !query.is_ascii() {
        return if s.name.contains(query) { 4 } else { 0 };
    }
    let (q, name) = (normalize(query), normalize(s.tname));
    if q.len() < 2 {
        return 0;
    }
    if name.starts_with(&q) {
        4
    } else if name.contains(&q) {
        3
    } else if q.len() >= 4 && distance(&q, &name[..name.len().min(q.len())]) <= 2 {
        2
    } else {
        0
    }
}

fn suggestions(query: &str, hizb: bool, sajdah: bool) -> Vec<(Focus, String)> {
    let m = &*MUSHAF;
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return vec![];
    }
    let number: Option<usize> = q.split(|c: char| !c.is_ascii_digit()).find(|t| !t.is_empty()).and_then(|t| t.parse().ok());
    let words = q.trim_matches(|c: char| c.is_ascii_digit() || c.is_whitespace());
    let wants_juz = words.starts_with("juz") || words.starts_with("jz") || words.contains("جزء");
    let wants_sajdah = words.starts_with("saj") || words.contains("سجد");
    let wants_hizb = words.starts_with("hiz") || words.starts_with("hz") || words.contains("حزب");

    let surah = (!wants_juz && !wants_sajdah && !wants_hizb)
        .then(|| (0..114).map(|i| (surah_score(i, &q, number), i)).filter(|&(sc, _)| sc > 0).max_by_key(|&(sc, i)| (sc, usize::MAX - i)))
        .flatten()
        .map(|(_, i)| i);
    let juz = match number {
        Some(n) if (1..=30).contains(&n) && !wants_sajdah && !wants_hizb => Some(n - 1),
        _ if wants_hizb => None,
        _ => surah.map(|i| m.juz.iter().position(|j| m.surahs[i].start < j.end - 1e-3).unwrap_or(29)),
    };
    let hizb = match number {
        Some(n) if hizb && wants_hizb && (1..=60).contains(&n) => Some(n - 1),
        _ => None,
    };
    let sajdah = if !sajdah { None } else { match number {
        Some(n) if wants_sajdah && (1..=15).contains(&n) => Some(n - 1),
        _ if wants_sajdah && number.is_none() => Some(0),
        _ => surah.and_then(|i| m.sajdah.iter().position(|sj| sj.surah as usize == i + 1)),
    } };

    let mut out = vec![];
    if let Some(i) = surah {
        let s = &m.surahs[i];
        out.push((Focus::Surah(i), format!("Surah {} · {} · {}", s.num, s.name, s.tname)));
    }
    if let Some(i) = juz {
        let j = &m.juz[i];
        out.push((Focus::Juz(i), format!("Juz {} · from {}:{}", j.num, j.from.0, j.from.1)));
    }
    if let Some(h) = hizb {
        let r = &m.rub[4 * h];
        out.push((Focus::Hizb(h), format!("Hizb {} · from {}:{}", h + 1, r.from.0, r.from.1)));
    }
    if let Some(i) = sajdah {
        let sj = &m.sajdah[i];
        out.push((Focus::Sajdah(i), format!("Sajdah {} · {}:{}", i + 1, sj.surah, sj.ayah)));
    }
    out
}

#[component]
pub fn Search(selected: Signal<Option<Focus>>, hizb: bool, placeholder: String) -> Element {
    let mut query = use_signal(String::new);
    let mut active = use_signal(|| 0usize);
    let mut open = use_signal(|| false);
    let items = suggestions(&query(), hizb, crate::SHOW_SAJDAH());
    let count = items.len();
    let mut choose = move |f: Focus| {
        selected.set(Some(f));
        query.set(String::new());
        open.set(false);
    };
    let first = items.get(active().min(count.saturating_sub(1))).map(|it| it.0);

    rsx! {
        div { class: "search",
            input {
                r#type: "search",
                dir: "auto",
                placeholder: "{placeholder}",
                value: "{query}",
                oninput: move |e| {
                    query.set(e.value());
                    active.set(0);
                    open.set(true);
                },
                onfocus: move |_| open.set(true),
                onblur: move |_| open.set(false),
                onkeydown: move |e| match e.key() {
                    Key::ArrowDown if count > 0 => active.set((active() + 1) % count),
                    Key::ArrowUp if count > 0 => active.set((active() + count - 1) % count),
                    Key::Enter => {
                        if let Some(f) = first {
                            choose(f);
                        }
                    }
                    Key::Escape => open.set(false),
                    _ => {}
                },
            }
            if open() && count > 0 {
                ul { class: "suggest", dir: "ltr",
                    for (n, (f, label)) in items.into_iter().enumerate() {
                        li {
                            class: if n == active() { "on" },
                            onmousedown: move |e| {
                                e.prevent_default();
                                choose(f);
                            },
                            onmouseenter: move |_| active.set(n),
                            "{label}"
                        }
                    }
                }
            }
        }
    }
}

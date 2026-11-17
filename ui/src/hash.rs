use crate::overlay::AyahKey;
use crate::Focus;

fn pairs() -> Vec<(String, String)> {
    let hash = web_sys::window().and_then(|w| w.location().hash().ok()).unwrap_or_default();
    hash.trim_start_matches('#')
        .split('&')
        .filter_map(|p| p.split_once('='))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

pub fn get(key: &str) -> Option<String> {
    pairs().into_iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

/// Replaces the given keys in the URL hash without adding history entries.
pub fn set(updates: &[(&str, Option<String>)]) {
    let mut current = pairs();
    for (key, value) in updates {
        match (current.iter().position(|(k, _)| k == key), value) {
            (Some(i), Some(v)) => current[i].1 = v.clone(),
            (Some(i), None) => {
                current.remove(i);
            }
            (None, Some(v)) => current.push((key.to_string(), v.clone())),
            (None, None) => {}
        }
    }
    let hash: Vec<String> = current.iter().map(|(k, v)| format!("{k}={v}")).collect();
    let Some(window) = web_sys::window() else { return };
    let path = window.location().pathname().unwrap_or_default();
    let url = if hash.is_empty() { path } else { format!("{path}#{}", hash.join("&")) };
    if let Ok(history) = window.history() {
        let _ = history.replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&url));
    }
}

pub fn parse_ayah(text: &str) -> Option<AyahKey> {
    let (s, a) = text.split_once(':')?;
    Some((s.parse().ok()?, a.parse().ok()?))
}

pub fn read_focus() -> Option<Focus> {
    let number = |k: &str| get(k).and_then(|v| v.parse::<usize>().ok()).filter(|&n| n >= 1);
    number("surah")
        .filter(|&n| n <= 114)
        .map(|n| Focus::Surah(n - 1))
        .or_else(|| number("juz").filter(|&n| n <= 30).map(|n| Focus::Juz(n - 1)))
        .or_else(|| number("sajdah").filter(|&n| n <= crate::MUSHAF.sajdah.len()).map(|n| Focus::Sajdah(n - 1)))
}

pub fn write_focus(focus: Option<Focus>) {
    let (surah, juz, sajdah) = match focus {
        Some(Focus::Surah(i)) => (Some(i + 1), None, None),
        Some(Focus::Juz(i)) => (None, Some(i + 1), None),
        Some(Focus::Sajdah(i)) => (None, None, Some(i + 1)),
        None => (None, None, None),
    };
    let text = |n: Option<usize>| n.map(|n| n.to_string());
    set(&[("surah", text(surah)), ("juz", text(juz)), ("sajdah", text(sajdah))]);
}

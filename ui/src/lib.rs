mod labels;
pub mod menu;
pub mod overlay;
mod progress;
mod search;
mod track;

use dioxus::prelude::*;
pub use quran_core::{data, model, MUSHAF};
use track::{span_of, Track};


#[derive(Clone, Copy, PartialEq)]
enum Layout {
    Full,
    Three,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Focus {
    Surah(usize),
    Juz(usize),
    Sajdah(usize),
}

#[component]
pub fn Page(title: Element, ruku: bool, #[props(default = VNode::empty())] extra: Element) -> Element {
    let mut layout = use_signal(|| Layout::Three);
    let focus = use_signal(|| None::<Focus>);
    let selected = use_signal(|| None::<Focus>);
    use_context_provider(|| overlay::Selection { focus, selected });
    let progress = progress::use_progress_provider();
    menu::use_menu_provider();
    use_future(move || async move {
        let mut ui = document::eval(include_str!("ui.js"));
        while let Ok((index, pos)) = ui.recv::<(i32, f32)>().await {
            progress.drag_bookmark(usize::try_from(index).ok(), pos);
        }
    });
    let rows: &[(usize, usize)] = match layout() {
        Layout::Full => &[(0, 30)],
        Layout::Three => &[(0, 10), (10, 20), (20, 30)],
    };
    rsx! {
        style { {include_str!("../assets/main.css")} }
        main { dir: "rtl",
            header {
                {title}
                search::Search { selected }
                div { class: "toggle", dir: "ltr",
                    button { class: if layout() == Layout::Full { "on" }, onclick: move |_| layout.set(Layout::Full), "Full 1–30" }
                    button { class: if layout() == Layout::Three { "on" }, onclick: move |_| layout.set(Layout::Three), "3 lines × 10 juz" }
                }
            }
            for &(a, b) in rows {
                Track {
                    key: "{a}-{b}",
                    lo: MUSHAF.juz[a].start,
                    hi: MUSHAF.juz[b - 1].end,
                    focus,
                    selected,
                    detail: false,
                    ruku: false,
                }
            }
            Detail { focus, selected, ruku, extra }
            menu::ContextMenu {}
        }
    }
}

pub fn tip(i: usize) -> String {
    let s = &MUSHAF.surahs[i];
    format!("{} {} ({})", s.num, s.name, s.tname)
}

fn isolate(s: &str) -> String {
    format!("\u{2068}{s}\u{2069}")
}

fn juz_of(pos: f32) -> usize {
    MUSHAF.juz.iter().position(|j| pos < j.end).unwrap_or(29) + 1
}

#[component]
fn Info(focus: Option<Focus>, selected: bool) -> Element {
    let m = &*MUSHAF;
    let text = match focus {
        None => {
            return rsx! {
                p { class: "info msg",
                    span { class: "idle", "Hover a surah, juz or sajdah mark. Click or tap to see details here." }
                    span { class: "hov", "Click or tap to see details here." }
                }
            };
        }
        Some(Focus::Surah(i)) => {
            let s = &m.surahs[i];
            let (j1, j2) = (juz_of(s.start), juz_of(s.end - 1e-3));
            let juz = if j1 == j2 { format!("Juz {j1}") } else { format!("Juz {j1}–{j2}") };
            format!(
                "{} · {} · {} · {} ayat · {} · pages {}–{} · {}",
                s.num, isolate(s.name), s.tname, s.ayat,
                if s.makki { "Makki" } else { "Madani" },
                s.start.floor() as u32 + 1, s.end.ceil() as u32, juz
            )
        }
        Some(Focus::Juz(i)) => {
            let j = &m.juz[i];
            let s = &m.surahs[j.from.0 as usize - 1];
            format!(
                "Juz {} · starts {} {}:{} · pages {}–{}",
                j.num, isolate(s.name), j.from.0, j.from.1, j.start.floor() as u32 + 1, j.end.ceil() as u32
            )
        }
        Some(Focus::Sajdah(i)) => {
            let sj = &m.sajdah[i];
            let s = &m.surahs[sj.surah as usize - 1];
            format!(
                "Sajdah {} of 15 · {} {}:{} · page {} · Juz {}",
                i + 1, isolate(s.name), sj.surah, sj.ayah, sj.pos.floor() as u32 + 1, juz_of(sj.pos)
            )
        }
    };
    rsx! {
        p { class: "info",
            "{text}"
            if selected { Actions {} }
        }
    }
}

#[component]
fn Actions() -> Element {
    let mut edit = use_context::<progress::Progress>().edit;
    rsx! {
        span { class: "actions",
            button { class: if edit() { "edit on" } else { "edit" }, onclick: move |_| edit.toggle(),
                if edit() { "Done editing" } else { "Mark progress" }
            }
        }
    }
}

#[component]
fn Detail(focus: Signal<Option<Focus>>, selected: Signal<Option<Focus>>, ruku: bool, extra: Element) -> Element {
    let range = selected().map(|f| match f {
        Focus::Sajdah(i) => span_of(Focus::Surah(MUSHAF.sajdah[i].surah as usize - 1)),
        _ => span_of(f),
    });
    rsx! {
        section {
            class: format!("detail{}{}", if range.is_some() { " on" } else { "" }, if focus().is_some() { " hovering" } else { "" }),
            Info { focus: selected(), selected: selected().is_some() }
            for (lo, hi) in range {
                Track { key: "{lo}-{hi}", lo, hi, focus, selected, detail: true, ruku }
            }
            {extra}
        }
    }
}

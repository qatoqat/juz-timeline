use crate::progress::{page_at, Mark, Progress};
use crate::MUSHAF;
use dioxus::prelude::*;

const MENU_W: f64 = 220.0;
const ITEM_H: f64 = 32.0;

#[derive(Clone, Copy, PartialEq)]
pub enum Target {
    Surah(usize),
    Juz(usize),
    Rub(usize),
    Sajdah(usize),
    Bookmark(usize),
    Ayah(u8, u16),
}

#[derive(Clone, Copy, PartialEq)]
pub struct Menu {
    pub x: f64,
    pub y: f64,
    pub target: Target,
}

pub fn use_menu_provider() -> Signal<Option<Menu>> {
    use_context_provider(|| Signal::new(None::<Menu>))
}

pub fn open_menu(mut menu: Signal<Option<Menu>>, e: &MouseEvent, target: Target) {
    e.prevent_default();
    let p = e.client_coordinates();
    menu.set(Some(Menu { x: p.x, y: p.y, target }));
}

fn mark(t: Target) -> Option<Mark> {
    match t {
        Target::Surah(i) => Some(Mark::Surah(i)),
        Target::Juz(i) => Some(Mark::Juz(i)),
        Target::Rub(i) => Some(Mark::Rub(i)),
        _ => None,
    }
}

fn describe(t: Target, progress: Progress) -> (String, String, u16) {
    let m = &*MUSHAF;
    match t {
        Target::Surah(i) => {
            let s = &m.surahs[i];
            (format!("Surah {} · {}", s.num, s.name), format!("https://quran.com/{}", s.num), page_at(s.start))
        }
        Target::Juz(i) => {
            let j = &m.juz[i];
            (format!("Juz {}", j.num), format!("https://quran.com/juz/{}", j.num), page_at(j.start))
        }
        Target::Rub(i) => {
            let r = &m.rub[i];
            let name = format!("Hizb {} {}", r.hizb(), r.quarter());
            (name.trim().to_string(), format!("https://quran.com/{}/{}", r.from.0, r.from.1), page_at(r.pos))
        }
        Target::Sajdah(i) => {
            let sj = &m.sajdah[i];
            (format!("Sajdah {} · {}:{}", i + 1, sj.surah, sj.ayah), format!("https://quran.com/{}/{}", sj.surah, sj.ayah), page_at(sj.pos))
        }
        Target::Ayah(s, a) => {
            let name = m.surahs[s as usize - 1].name;
            let page = page_at(quran_core::model::page_pos(s, a));
            (format!("{s}:{a} · {name} · page {page}"), format!("https://quran.com/{s}/{a}"), page)
        }
        Target::Bookmark(i) => {
            let page = progress.bookmarks.peek().get(i).copied().unwrap_or(1);
            (format!("Bookmark · page {page}"), format!("https://quran.com/page/{page}"), page)
        }
    }
}

fn open_url(url: &str) {
    if let Some(w) = web_sys::window() {
        let _ = w.open_with_url_and_target(url, "_blank");
    }
}

#[component]
pub fn ContextMenu() -> Element {
    let mut menu = use_context::<Signal<Option<Menu>>>();
    let progress = use_context::<Progress>();
    let Some(Menu { x, y, target }) = menu() else { return rsx! {} };
    let (title, url, page) = describe(target, progress);
    let done = mark(target).map(|m| progress.done.read().has(m));
    let has_bookmarks = !progress.bookmarks.read().is_empty();
    let existing = match target {
        Target::Bookmark(i) => Some(i),
        _ => progress.bookmarks.read().iter().position(|&p| p == page),
    };
    let rows = 2 + done.is_some() as usize + if existing.is_some() { 1 } else { 1 + has_bookmarks as usize };
    let (vw, vh) = web_sys::window()
        .and_then(|w| Some((w.inner_width().ok()?.as_f64()?, w.inner_height().ok()?.as_f64()?)))
        .unwrap_or((1200.0, 800.0));
    let left = x.min(vw - MENU_W - 6.0).max(6.0);
    let top = y.min(vh - rows as f64 * ITEM_H - 16.0).max(6.0);
    let mut close = move || menu.set(None);

    rsx! {
        div {
            class: "menu-shade",
            onclick: move |_| close(),
            oncontextmenu: move |e| {
                e.prevent_default();
                close();
            },
        }
        div { class: "menu", dir: "ltr", style: "left:{left}px;top:{top}px;width:{MENU_W}px",
            div { class: "menu-title", "{title}" }
            button {
                onclick: move |_| {
                    open_url(&url);
                    close();
                },
                "Open in quran.com"
            }
            if let (Some(is_done), Some(m)) = (done, mark(target)) {
                button {
                    onclick: move |_| {
                        progress.toggle(m);
                        close();
                    },
                    if is_done { "Unmark done" } else { "Mark as done" }
                }
            }
            if let Some(i) = existing {
                button {
                    class: "danger",
                    onclick: move |_| {
                        progress.remove_bookmark(i);
                        close();
                    },
                    if matches!(target, Target::Bookmark(_)) { "Remove bookmark" } else { "Remove bookmark on page {page}" }
                }
            } else {
                button {
                    onclick: move |_| {
                        progress.add_bookmark(page);
                        close();
                    },
                    "Add bookmark here"
                }
                if has_bookmarks {
                    button {
                        onclick: move |_| {
                            progress.move_active_bookmark(page);
                            close();
                        },
                        "Move bookmark here"
                    }
                }
            }
        }
    }
}

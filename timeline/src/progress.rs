use crate::Focus;
use dioxus::prelude::*;
use std::collections::BTreeSet;

const DONE_KEY: &str = "quran-timeline:done";
const BOOKMARK_KEY: &str = "quran-timeline:bookmark";
pub const LAST_PAGE: u16 = 604;

#[derive(Clone, Copy, PartialEq)]
pub enum Mark {
    Surah(usize),
    Juz(usize),
    Rub(usize),
}

impl Mark {
    pub fn of(f: Focus) -> Option<Mark> {
        match f {
            Focus::Surah(i) => Some(Mark::Surah(i)),
            Focus::Juz(i) => Some(Mark::Juz(i)),
            Focus::Sajdah(_) => None,
        }
    }
}

#[derive(Clone, Default, PartialEq)]
pub struct Done {
    surah: BTreeSet<usize>,
    juz: BTreeSet<usize>,
    rub: BTreeSet<usize>,
}

impl Done {
    fn set(&self, m: Mark) -> (&BTreeSet<usize>, usize) {
        match m {
            Mark::Surah(i) => (&self.surah, i),
            Mark::Juz(i) => (&self.juz, i),
            Mark::Rub(i) => (&self.rub, i),
        }
    }

    pub fn has(&self, m: Mark) -> bool {
        let (set, i) = self.set(m);
        set.contains(&i)
    }

    pub fn toggle(&mut self, m: Mark) {
        let (set, i) = match m {
            Mark::Surah(i) => (&mut self.surah, i),
            Mark::Juz(i) => (&mut self.juz, i),
            Mark::Rub(i) => (&mut self.rub, i),
        };
        if !set.remove(&i) {
            set.insert(i);
        }
    }

    fn encode(&self) -> String {
        let list = |s: &BTreeSet<usize>| s.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(",");
        format!("s={}|j={}|r={}", list(&self.surah), list(&self.juz), list(&self.rub))
    }

    fn decode(text: &str) -> Done {
        let mut done = Done::default();
        for part in text.split('|') {
            let Some((kind, items)) = part.split_once('=') else { continue };
            let set = match kind {
                "s" => &mut done.surah,
                "j" => &mut done.juz,
                "r" => &mut done.rub,
                _ => continue,
            };
            set.extend(items.split(',').filter_map(|i| i.parse::<usize>().ok()));
        }
        done
    }
}

#[derive(Clone, Copy)]
pub struct Progress {
    pub done: Signal<Done>,
    pub edit: Signal<bool>,
    pub bookmarks: Signal<Vec<u16>>,
    pub active: Signal<usize>,
}

pub fn page_at(pos: f32) -> u16 {
    (pos.floor() as i32 + 1).clamp(1, LAST_PAGE as i32) as u16
}

impl Progress {

    pub fn toggle(mut self, m: Mark) {
        self.done.with_mut(|d| d.toggle(m));
        store(DONE_KEY, &self.done.peek().encode());
    }

    fn save_bookmarks(self) {
        let text = self.bookmarks.peek().iter().map(|p| p.to_string()).collect::<Vec<_>>().join(",");
        store(BOOKMARK_KEY, &text);
    }

    pub fn drag_bookmark(mut self, index: Option<usize>, pos: f32) {
        let page = page_at(pos);
        let count = self.bookmarks.peek().len();
        match index.filter(|&i| i < count) {
            Some(i) => self.active.set(i),
            None if count == 0 => return self.add_bookmark(page),
            None => {}
        }
        let i = *self.active.peek();
        if self.bookmarks.peek().get(i) != Some(&page) {
            self.bookmarks.with_mut(|b| b[i] = page);
            self.save_bookmarks();
        }
    }

    pub fn add_bookmark(mut self, page: u16) {
        self.bookmarks.with_mut(|b| b.push(page));
        self.active.set(self.bookmarks.peek().len() - 1);
        self.save_bookmarks();
    }

    pub fn move_active_bookmark(mut self, page: u16) {
        let i = *self.active.peek();
        if i < self.bookmarks.peek().len() {
            self.bookmarks.with_mut(|b| b[i] = page);
            self.save_bookmarks();
        } else {
            self.add_bookmark(page);
        }
    }

    pub fn remove_bookmark(mut self, i: usize) {
        self.bookmarks.with_mut(|b| {
            if i < b.len() {
                b.remove(i);
            }
        });
        let len = self.bookmarks.peek().len();
        self.active.set(len.saturating_sub(1));
        self.save_bookmarks();
    }
}

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

fn load(key: &str) -> Option<String> {
    storage()?.get_item(key).ok()?
}

fn store(key: &str, value: &str) {
    if let Some(s) = storage() {
        let _ = s.set_item(key, value);
    }
}

pub fn use_progress_provider() -> Progress {
    let done = use_signal(|| load(DONE_KEY).map(|t| Done::decode(&t)).unwrap_or_default());
    let edit = use_signal(|| false);
    let bookmarks = use_signal(|| {
        load(BOOKMARK_KEY)
            .map(|t| t.split(',').filter_map(|p| p.parse::<u16>().ok()).filter(|p| (1..=LAST_PAGE).contains(p)).collect::<Vec<u16>>())
            .unwrap_or_default()
    });
    let active = use_signal(move || bookmarks.peek().len().saturating_sub(1));
    use_context_provider(|| Progress { done, edit, bookmarks, active })
}

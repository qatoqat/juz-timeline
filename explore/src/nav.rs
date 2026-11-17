use dioxus::prelude::*;
use quran_ui::overlay::{ayah_pos, range_of, AyahKey, Overlay, Selection};

#[derive(Clone, Copy)]
pub struct Nav {
    history: Signal<Vec<(AyahKey, usize)>>,
    cursor: Signal<usize>,
    restoring: Signal<bool>,
    root: Signal<Option<usize>>,
    pending: Signal<Option<usize>>,
}

impl Nav {
    pub fn new(root: Signal<Option<usize>>, pending: Signal<Option<usize>>) -> Nav {
        Nav { history: Signal::new(vec![]), cursor: Signal::new(0), restoring: Signal::new(false), root, pending }
    }

    pub fn record(mut self, key: AyahKey, family: usize) {
        if *self.restoring.peek() {
            self.restoring.set(false);
            return;
        }
        let mut history = self.history.peek().clone();
        let cursor = *self.cursor.peek();
        if history.get(cursor) == Some(&(key, family)) {
            return;
        }
        history.truncate(if history.is_empty() { 0 } else { cursor + 1 });
        history.push((key, family));
        self.cursor.set(history.len() - 1);
        self.history.set(history);
    }

    pub fn keep(mut self, family: usize) {
        self.pending.set(Some(family));
    }

    pub fn can(self, delta: i32) -> bool {
        let target = (self.cursor)() as i32 + delta;
        target >= 0 && (target as usize) < self.history.read().len()
    }

    pub fn position(self) -> (usize, usize) {
        ((self.cursor)() + 1, self.history.read().len())
    }

    pub fn go(mut self, delta: i32, overlay: Overlay, selection: Selection) {
        if !self.can(delta) {
            return;
        }
        let target = (*self.cursor.peek() as i32 + delta) as usize;
        let (key, family) = self.history.peek()[target];
        self.cursor.set(target);
        self.restoring.set(true);
        let mut picked = overlay.picked;
        if *picked.peek() == Some(key) {
            self.root.set(Some(family));
            return;
        }
        self.pending.set(Some(family));
        let pos = ayah_pos(key);
        let inside = (*selection.selected.peek()).map(range_of).is_some_and(|(lo, hi)| pos >= lo && pos < hi);
        if inside {
            picked.set(Some(key));
        } else {
            selection.show(key, overlay);
        }
    }
}

#[component]
pub fn NavBar() -> Element {
    let nav = use_context::<Nav>();
    let overlay = use_context::<Overlay>();
    let selection = use_context::<Selection>();
    let (at, total) = nav.position();
    rsx! {
        div { class: "navbar",
            button { class: "nav", disabled: !nav.can(-1), "data-tip": "Back", onclick: move |_| nav.go(-1, overlay, selection), "‹ Back" }
            button { class: "nav", disabled: !nav.can(1), "data-tip": "Forward", onclick: move |_| nav.go(1, overlay, selection), "Forward ›" }
            if total > 1 { span { class: "nav-pos", "{at} / {total}" } }
        }
    }
}

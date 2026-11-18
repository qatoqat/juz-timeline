use dioxus::prelude::*;

/// Back/forward over visited states; `record` ignores the change that `go` itself caused.
#[derive(Clone, Copy)]
pub struct Trail<T: Copy + PartialEq + 'static> {
    list: Signal<Vec<T>>,
    cursor: Signal<usize>,
    restoring: Signal<bool>,
}

impl<T: Copy + PartialEq + 'static> Trail<T> {
    pub fn new() -> Self {
        Trail { list: Signal::new(vec![]), cursor: Signal::new(0), restoring: Signal::new(false) }
    }

    pub fn record(mut self, value: T) {
        if *self.restoring.peek() {
            self.restoring.set(false);
            return;
        }
        let mut list = self.list.peek().clone();
        let cursor = *self.cursor.peek();
        if list.get(cursor) == Some(&value) {
            return;
        }
        list.truncate(if list.is_empty() { 0 } else { cursor + 1 });
        list.push(value);
        self.cursor.set(list.len() - 1);
        self.list.set(list);
    }

    pub fn can(self, delta: i32) -> bool {
        let target = (self.cursor)() as i32 + delta;
        target >= 0 && (target as usize) < self.list.read().len()
    }

    /// Moves the cursor and returns the state to restore.
    pub fn go(mut self, delta: i32) -> Option<T> {
        if !self.can(delta) {
            return None;
        }
        let target = (*self.cursor.peek() as i32 + delta) as usize;
        self.cursor.set(target);
        self.restoring.set(true);
        Some(self.list.peek()[target])
    }
}

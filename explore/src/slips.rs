use crate::relations::Twin;
use std::collections::BTreeMap;

const KEY: &str = "quran-timeline:slips";
const RIGHT_TO_CLEAR: u8 = 2;

/// Look-alike passages the reader slipped on, kept in this browser: twin key -> right answers still needed.
#[derive(Clone, PartialEq, Default)]
pub struct Slips(BTreeMap<String, u8>);

pub fn twin_key(t: &Twin) -> String {
    format!("{}-{}", t.sides[0].0, t.sides[1].0)
}

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

impl Slips {
    pub fn load() -> Slips {
        let text = storage().and_then(|s| s.get_item(KEY).ok()?).unwrap_or_default();
        Slips(text.split(',').filter_map(|p| p.split_once(':')).filter_map(|(k, n)| Some((k.to_string(), n.parse().ok()?))).collect())
    }

    fn save(&self) {
        if let Some(s) = storage() {
            let text: Vec<String> = self.0.iter().map(|(k, n)| format!("{k}:{n}")).collect();
            let _ = s.set_item(KEY, &text.join(","));
        }
    }

    pub fn has(&self, key: &str) -> bool {
        self.0.contains_key(key)
    }

    pub fn answer(&mut self, key: String, right: bool) {
        if !right {
            self.0.insert(key, RIGHT_TO_CLEAR);
        } else if let Some(n) = self.0.get_mut(&key) {
            *n -= 1;
            if *n == 0 {
                self.0.remove(&key);
            }
        }
        self.save();
    }
}

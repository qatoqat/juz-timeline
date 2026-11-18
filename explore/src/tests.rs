//! Renders every view the Similar ayah page can reach, so a code or data change that panics fails here
//! instead of in the browser. Needs explore/assets/similar.json (tools/rebuild_data.sh).
use crate::data::Data;
use crate::nav::Nav;
use crate::similar::Panel;
use crate::spine::{column, row, Cell};
use crate::tree::{ayah_label, Shared};
use dioxus::prelude::*;
use quran_ui::menu::Menu;
use quran_ui::overlay::{AyahKey, Overlay, Selection};
use quran_ui::{Focus, MUSHAF};
use std::cell::RefCell;
use std::panic::{self, AssertUnwindSafe};
use std::rc::Rc;
use std::sync::Once;

const TREE: &str = r#"class="tree""#;

thread_local! {
    static PANICS: RefCell<Vec<String>> = const { RefCell::new(vec![]) };
}

fn data() -> Rc<Data> {
    static HOOK: Once = Once::new();
    HOOK.call_once(|| {
        let default = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            PANICS.with(|p| p.borrow_mut().push(info.to_string()));
            default(info);
        }));
    });
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/similar.json");
    let json = std::fs::read_to_string(path).expect("explore/assets/similar.json is missing: run tools/rebuild_data.sh");
    Rc::new(Data::parse(&json).expect("similar.json does not parse"))
}

#[component]
fn Host(data: Shared, picked: Option<AyahKey>, selected: Option<Focus>, family: Option<usize>) -> Element {
    use_context_provider(|| Overlay { dots: Signal::new(vec![]), marks: Signal::new(vec![]), picked: Signal::new(picked), hot: Signal::new(vec![]) });
    use_context_provider(|| Selection { focus: Signal::new(None), selected: Signal::new(selected) });
    use_context_provider(|| Signal::new(None::<Menu>));
    let root = use_signal(move || family);
    let pending = use_signal(|| None);
    use_context_provider(|| Nav::new(root, pending));
    rsx! { Panel { data: Some(data), root, pending } }
}

/// The page's HTML, or every panic raised while drawing it (including any the renderer swallowed).
fn render(data: &Rc<Data>, picked: Option<AyahKey>, selected: Option<Focus>, family: Option<usize>) -> Result<String, String> {
    PANICS.with(|p| p.borrow_mut().clear());
    let html = panic::catch_unwind(AssertUnwindSafe(|| {
        let mut dom = VirtualDom::new_with_props(Host, HostProps { data: Shared(data.clone()), picked, selected, family });
        dom.rebuild_in_place();
        dioxus_ssr::render(&dom)
    }));
    let panics = PANICS.with(|p| p.take());
    match html {
        Ok(html) if panics.is_empty() => Ok(html),
        _ => Err(panics.join(" | ")),
    }
}

fn open(data: &Rc<Data>, ayah: usize, family: Option<usize>) -> Result<String, String> {
    let key = data.keys[ayah];
    render(data, Some(key), Some(Focus::Surah(key.0 as usize - 1)), family)
}

fn report(what: &str, checked: usize, failures: Vec<String>) {
    assert!(checked > 0, "{what}: nothing was checked");
    let shown: Vec<&str> = failures.iter().take(20).map(String::as_str).collect();
    assert!(failures.is_empty(), "{what}: {} of {checked} failed:\n{}", failures.len(), shown.join("\n"));
}

/// Every ayah with every shared phrase it can branch from (the spine's bars).
#[test]
fn every_tree_renders() {
    let d = data();
    let (mut checked, mut failures) = (0, vec![]);
    for a in 0..d.keys.len() {
        let families: Vec<Option<usize>> = match d.family_of[a].as_slice() {
            [] => vec![None],
            list => list.iter().copied().map(Some).collect(),
        };
        for f in families {
            checked += 1;
            match open(&d, a, f) {
                Ok(html) if f.is_some() && !html.contains(TREE) => failures.push(format!("{} family {f:?}: no tree drawn", ayah_label(&d, a))),
                Ok(_) => {}
                Err(e) => failures.push(format!("{} family {f:?}: {e}", ayah_label(&d, a))),
            }
        }
    }
    report("trees", checked, failures);
}

/// `root` catches up with a new pick one render later, so the page first draws the new ayah with the old family:
/// the next ayah, a late ayah's ref, a close variant, Back/Forward, and every family with the shortest ayah.
#[test]
fn stale_root_renders() {
    let d = data();
    let shortest = (0..d.keys.len()).min_by_key(|&a| d.words(a).len()).expect("no ayat");
    let mut cases: Vec<(usize, usize)> = (0..d.families.len()).map(|f| (shortest, f)).collect();
    for a in 0..d.keys.len() {
        let Some(f) = d.default_root(a) else { continue };
        if a + 1 < d.keys.len() {
            cases.push((a + 1, f));
        }
        cases.extend(d.late_ayat(f, a).into_iter().map(|l| (l, f)));
        for &v in &d.variant_of[a] {
            cases.extend(d.variants[v].members.iter().map(|m| (m.0, f)));
        }
    }
    let mut failures = vec![];
    for &(a, f) in &cases {
        let member = d.family_of[a].contains(&f);
        match open(&d, a, Some(f)) {
            Ok(html) if html.contains(TREE) != member => failures.push(format!("{} family {f}: tree drawn = {}, member = {member}", ayah_label(&d, a), !member)),
            Ok(_) => {}
            Err(e) => failures.push(format!("{} family {f}: {e}", ayah_label(&d, a))),
        }
    }
    report("stale roots", cases.len(), failures);
}

/// The ayah text row of every ayah, folded and with every fold open: each word in reach once, in order, and every
/// phrase's bar on words of its own.
#[test]
fn every_row_lays_out() {
    let d = data();
    let mut failures = vec![];
    for a in 0..d.keys.len() {
        let spans: Vec<(usize, usize)> = d.family_of[a].iter().map(|&f| d.span_in(f, a)).collect();
        let total = d.words(a).len();
        let (_, _, folded) = row(&spans, total, &[]);
        let starts: Vec<usize> = folded.iter().filter_map(|c| if let Cell::Fold(s, _) = c { Some(*s) } else { None }).collect();
        for open in [vec![], starts] {
            let (lo, hi, cells) = row(&spans, total, &open);
            let covered: Vec<usize> = cells
                .iter()
                .flat_map(|&c| match c {
                    Cell::Word(i) => i..i + 1,
                    Cell::Fold(s, e) => s..e,
                    Cell::Unfold(_) => 0..0,
                })
                .collect();
            if covered != (lo..hi).collect::<Vec<_>>() {
                failures.push(format!("{} open {open:?}: words {covered:?}, expected {lo}..{hi}", ayah_label(&d, a)));
            }
            if let Some(&(s, e)) = spans.iter().find(|&&(s, e)| column(&cells, s).zip(column(&cells, e - 1)).is_none_or(|(x, y)| y - x != e - 1 - s)) {
                failures.push(format!("{} open {open:?}: phrase {s}..{e} is not on words of its own", ayah_label(&d, a)));
            }
        }
    }
    report("text rows", d.keys.len(), failures);
}

/// The ruku' list of every surah and juz.
#[test]
fn ruku_lists_render() {
    let d = data();
    let focuses: Vec<Focus> = (0..MUSHAF.surahs.len()).map(Focus::Surah).chain((0..30).map(Focus::Juz)).collect();
    let mut failures = vec![];
    for &focus in &focuses {
        if let Err(e) = render(&d, None, Some(focus), None) {
            failures.push(e);
        }
    }
    report("ruku' lists", focuses.len(), failures);
}

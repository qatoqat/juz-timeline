use dioxus::prelude::*;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        quran_ui::Page {
            ruku: false,
            title: rsx! {
                h1 {
                    "خط الأجزاء"
                    span { class: "sub", "Juz timeline" }
                    a { class: "explore", href: "explore/", "Explore" }
                }
            },
        }
        quran_ui::Stale {}
    }
}

use dioxus::prelude::*;

use crate::pwa::PwaHead;
use crate::storage;
use crate::ui::screens::{AppLayout, Screen};

#[component]
pub fn App() -> Element {
    let state = use_signal(storage::load_state);
    let screen = use_signal(|| Screen::Dashboard);
    let feedback = use_signal(|| None::<String>);

    rsx! {
        PwaHead {}
        AppLayout { state, screen, feedback }
    }
}

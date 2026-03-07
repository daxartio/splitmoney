mod app;
mod csv;
mod currencies;
mod pwa;
mod state;
mod storage;
mod ui;

use app::App;

fn main() {
    dioxus::launch(App);
}

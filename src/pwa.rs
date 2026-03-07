use dioxus::prelude::*;

#[component]
pub fn PwaHead() -> Element {
    rsx! {
        document::Stylesheet { href: asset!("/assets/main.css") }
        document::Meta { name: "theme-color", content: "#0d4a48" }
        document::Meta { name: "application-name", content: "SplitMoney Lite" }
        document::Meta { name: "apple-mobile-web-app-capable", content: "yes" }
        document::Meta { name: "apple-mobile-web-app-status-bar-style", content: "black-translucent" }
        document::Meta { name: "apple-mobile-web-app-title", content: "SplitMoney Lite" }
        document::Link { rel: "apple-touch-icon", sizes: "180x180", href: asset!("/assets/apple-touch-icon.png") }
        document::Link { rel: "icon", sizes: "192x192", href: asset!("/assets/icon-192.png"), r#type: "image/png" }
        document::Link { rel: "manifest", href: asset!("/assets/manifest.json") }
        document::Script { src: asset!("/assets/sw-register.js") }
    }
}

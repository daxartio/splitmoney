You are an expert [Dioxus 0.7](https://dioxuslabs.com/learn/0.7) assistant for this repository.

Use only Dioxus 0.7 APIs (`cx`, `Scope`, `use_state` are not used here).

# Web (Repository Specific)

- Platform: web-only (`dioxus = { version = "0.7.3", features = ["web"] }`).
- Entry point: `src/main.rs` -> `dioxus::launch(App)`.
- Root component: `src/app.rs`.
- Persistent storage: browser LocalStorage key `splitwise_lite_state_v1` in `src/storage.rs`.
- Base path is fixed to `/splitmoney/` (`Dioxus.toml` -> `[web.app].base_path`).
- Keep all examples and code changes compatible with `dx serve --platform web`.
- Use `asset!("/...")` for static files under `assets/`.

# PWA (Repository Specific)

- PWA head/meta setup is centralized in `src/pwa.rs` (`PwaHead` component).
- Manifest file: `assets/manifest.webmanifest`.
- Service worker: `assets/service-worker.js`.
- SW registration script: `assets/sw-register.js`.
- App icons:
  - `assets/icon-192.png`
  - `assets/icon-512.png`
  - `assets/apple-touch-icon.png`
- Current SW cache key: `splitmoney-cache-v1`.

When changing PWA/base path behavior, update all related places together:

- `Dioxus.toml` (`base_path`)
- `assets/manifest.webmanifest` (`id`, `start_url`, `scope`, icon paths)
- `assets/service-worker.js` (asset caching and scope assumptions)
- `src/pwa.rs` (manifest/script links in `<head>`)

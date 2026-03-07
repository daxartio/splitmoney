# SplitMoney Lite (Dioxus 0.7)

Browser-only Splitwise-lite app built in Rust + Dioxus, with all data persisted in LocalStorage.

## Features

- Participants management (add, rename, activate/deactivate)
- Expenses with icon, payer, split helpers, and manual share editing
- Full ISO currency code list in expense form + custom currency code input
- Last selected currency is persisted and reused automatically
- Currency-isolated accounting: balances and settlement suggestions are computed per currency
- Settlements (debt repayments)
- Balance dashboard + minimal transfer suggestions
- Unified history list with search
- CSV export/import with validation and error messaging
- Offline-ready PWA (manifest + service worker)

## Tech

- Rust + `dioxus = 0.7.1` (web target only)
- `serde` / `serde_json`
- `uuid`
- `csv`
- Browser LocalStorage key: `splitwise_lite_state_v1`

## Run in development

```bash
cargo install dioxus-cli
rustup target add wasm32-unknown-unknown
dx serve --platform web
```

## Build for web

```bash
dx build --platform web --release
```

Build output:

`target/dx/splitmoney/release/web/public/`

## PWA

Included assets:

- `assets/manifest.json`
- `assets/service-worker.js`
- `assets/sw-register.js`
- `assets/icon-192.png`, `assets/icon-512.png`, `assets/apple-touch-icon.png`

Install via browser "Install app" / "Add to Home Screen" after opening the deployed app.

## CSV

Use `Import/Export` screen.

- Export: downloads one CSV with `participant`, `expense`, and `settlement` rows.
- Import: validates headers and row values, ignores unknown columns safely.

Example import file:

- `assets/example-import.csv`

## Module layout

- `src/state.rs` - domain models + validation + balance math
- `src/storage.rs` - LocalStorage persistence
- `src/ui/screens.rs` - all screens and screen switching
- `src/csv.rs` - CSV export/import logic
- `src/pwa.rs` - `<head>` PWA tags and assets

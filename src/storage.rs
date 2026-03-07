use crate::state::AppState;

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub const STORAGE_KEY: &str = "splitwise_lite_state_v1";

#[cfg(target_arch = "wasm32")]
fn local_storage() -> Option<web_sys::Storage> {
    web_sys::window()
        .and_then(|window| window.local_storage().ok())
        .flatten()
}

pub fn load_state() -> AppState {
    #[cfg(target_arch = "wasm32")]
    {
        let Some(storage) = local_storage() else {
            return AppState::default();
        };

        let Ok(Some(payload)) = storage.get_item(STORAGE_KEY) else {
            return AppState::default();
        };

        serde_json::from_str::<AppState>(&payload)
            .map(|state| state.with_defaults())
            .unwrap_or_default()
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        AppState::default()
    }
}

pub fn save_state(state: &AppState) {
    #[cfg(target_arch = "wasm32")]
    {
        let Some(storage) = local_storage() else {
            return;
        };

        if let Ok(payload) = serde_json::to_string(state) {
            let _ = storage.set_item(STORAGE_KEY, &payload);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = state;
    }
}

pub fn reset_state() {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(storage) = local_storage() {
            let _ = storage.remove_item(STORAGE_KEY);
        }
    }
}

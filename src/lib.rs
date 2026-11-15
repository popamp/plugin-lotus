//! Lotus Bloom for Popamp: a flower whose rings of petals swell and shrink
//! with the music. The calls are described in
//! [popamp-plugin-api](https://github.com/popamp/popamp-plugin-api), the
//! scene in Popamp's docs/plugins.md.

pub mod pulse;

const FLOWER: &str = include_str!("flower.wgsl");

/// One pass straight to the window; the music arrives as `pulse`'s data.
pub fn scene_json() -> String {
    serde_json::json!({
        "data_bytes": pulse::DATA_BYTES,
        "passes": [{ "shader": FLOWER }],
    })
    .to_string()
}

#[cfg(target_arch = "wasm32")]
mod plugin {
    use crate::pulse::{Input, Pulse};
    use extism_pdk::*;
    use popamp_plugin_api::{Version, VERSION};
    use std::sync::Mutex;

    /// The pulse lives between frames; the player calls `frame` from one thread.
    static PULSE: Mutex<Option<Pulse>> = Mutex::new(None);

    #[plugin_fn]
    pub fn api_version(_: String) -> FnResult<Json<Version>> {
        Ok(Json(VERSION))
    }

    #[plugin_fn]
    pub fn scene(_: Vec<u8>) -> FnResult<String> {
        Ok(crate::scene_json())
    }

    #[plugin_fn]
    pub fn frame(bytes: Vec<u8>) -> FnResult<Vec<u8>> {
        let input = Input::parse(&bytes).ok_or_else(|| Error::msg(format!("a frame of {} bytes", bytes.len())))?;
        let mut pulse = PULSE.lock().unwrap_or_else(|e| e.into_inner());
        Ok(pulse.get_or_insert_with(Pulse::default).step(&input).to_vec())
    }
}

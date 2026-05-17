#[cfg(target_arch = "wasm32")]
#[path = "showcase.rs"]
mod showcase;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub async fn start_showcase() -> Result<(), wasm_bindgen::JsValue> {
    showcase::run_web().await
}

#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    eprintln!("showcase_web is intended for wasm32-unknown-unknown");
}

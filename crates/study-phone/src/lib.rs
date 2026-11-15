//! R211 spike: does the study core run in a browser? Opens the database in memory, makes a
//! topic, and says how many change lines that made, which is what the phone would send to
//! the account.

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn smoke() -> Result<String, JsValue> {
    let mut store = study_store::Store::in_memory().map_err(|error| JsValue::from_str(&error.to_string()))?;
    let lines = store.sync_collect().map_err(|error| JsValue::from_str(&error.to_string()))?;
    Ok(format!("in-memory database opened, {lines} change lines collected, device {}", store.device_id()))
}

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn the_core_runs_in_a_browser() {
        let said = super::smoke().unwrap();
        assert!(said.starts_with("in-memory database opened"), "{said}");
    }
}

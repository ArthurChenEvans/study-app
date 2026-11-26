//! R228: how the app looks (colour theme, dark or light, colour-blind palette), synced to
//! every device like card settings. Each device still keeps a copy for the first paint, so the
//! app opens in the right colours before the database is read.

use serde::{Deserialize, Serialize};

use crate::{ApiError, ApiResult, App};

const KEY: &str = "appearance";

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Appearance {
    /// A theme's id from the frontend's list (coffee, honey, miku...).
    pub colours: String,
    pub dark: bool,
    pub colour_blind: bool,
}

impl App {
    /// The synced appearance, or None until it's first chosen on any device.
    pub fn appearance(&self) -> ApiResult<Option<Appearance>> {
        Ok(self.store.setting_value(KEY)?.and_then(|json| serde_json::from_str(&json).ok()))
    }

    pub fn set_appearance(&mut self, appearance: Appearance) -> ApiResult<()> {
        let id = &appearance.colours;
        if id.is_empty() || id.len() > 32 || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return Err(ApiError::new("that isn't one of the colour themes"));
        }
        let json = serde_json::to_string(&appearance).map_err(ApiError::new)?;
        Ok(self.store.set_setting_value(KEY, &json)?)
    }
}

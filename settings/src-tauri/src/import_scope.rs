use overlay_renderer::config::OverlayConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportScope { Full, Appearance, Timing, Hotkeys, Presets, Surfaces }

pub fn merge(base: &OverlayConfig, imported: OverlayConfig, scope: ImportScope) -> Result<OverlayConfig, String> {
    let mut result = match scope {
        ImportScope::Full => imported,
        _ => base.clone(),
    };
    result.validate_for_save().map_err(|error| error.to_string())?;
    result.normalize();
    Ok(result)
}

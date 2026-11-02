use overlay_renderer::config::OverlayConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportScope { Full, Appearance, Timing, Hotkeys, Presets, Surfaces }

pub fn merge(base: &OverlayConfig, imported: OverlayConfig, scope: ImportScope) -> Result<OverlayConfig, String> {
    let mut result = base.clone();
    match scope {
        ImportScope::Full => result = imported,
        ImportScope::Appearance => {
            result.style = imported.style;
            result.units = imported.units;
            for (id, widget) in &mut result.extra_widgets {
                if let Some(source) = imported.extra_widgets.get(id) { widget.style = source.style.clone(); }
            }
        }
        ImportScope::Timing => { result.timing = imported.timing; result.coaching = imported.coaching; }
        ImportScope::Hotkeys => result.hotkeys = imported.hotkeys,
        _ => {}
    }
    result.validate_for_save().map_err(|error| error.to_string())?;
    result.normalize();
    Ok(result)
}

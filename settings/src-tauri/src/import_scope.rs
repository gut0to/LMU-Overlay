use overlay_renderer::config::OverlayConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportScope {
    Full,
    Appearance,
    Timing,
    Hotkeys,
    Presets,
    Surfaces,
}

pub fn merge(
    base: &OverlayConfig,
    imported: OverlayConfig,
    scope: ImportScope,
) -> Result<OverlayConfig, String> {
    let mut result = base.clone();
    match scope {
        ImportScope::Full => result = imported,
        ImportScope::Appearance => {
            result.style = imported.style;
            result.units = imported.units;
            for (id, widget) in &mut result.extra_widgets {
                if let Some(source) = imported.extra_widgets.get(id) {
                    widget.style = source.style.clone();
                }
            }
        }
        ImportScope::Timing => {
            result.timing = imported.timing;
            result.coaching = imported.coaching;
        }
        ImportScope::Hotkeys => result.hotkeys = imported.hotkeys,
        ImportScope::Presets => result.presets = imported.presets,
        ImportScope::Surfaces => {
            result.window = geometry(&base.window, &imported.window);
            result.layout = imported.layout;
            result.widgets = imported.widgets;
            result.overlays = imported
                .overlays
                .into_iter()
                .map(|mut layer| {
                    let previous = base
                        .overlays
                        .iter()
                        .find(|previous| previous.id == layer.id)
                        .map(|previous| &previous.window)
                        .unwrap_or(&base.window);
                    layer.window = geometry(previous, &layer.window);
                    layer
                })
                .collect();
            for (id, widget) in &mut result.extra_widgets {
                if let Some(source) = imported.extra_widgets.get(id) {
                    widget.layout = source.layout.clone();
                    widget.enabled = source.enabled;
                }
            }
        }
    }
    result
        .validate_for_save()
        .map_err(|error| error.to_string())?;
    result.normalize();
    Ok(result)
}

fn geometry(
    base: &overlay_renderer::config::WindowConfig,
    source: &overlay_renderer::config::WindowConfig,
) -> overlay_renderer::config::WindowConfig {
    let mut result = base.clone();
    result.x = source.x;
    result.y = source.y;
    result.width = source.width;
    result.height = source.height;
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn baseline() -> OverlayConfig {
        OverlayConfig::from_import_text(overlay_renderer::config::default_config_text()).unwrap()
    }
    #[test]
    fn unknown_scopes_and_invalid_merged_bindings_are_rejected() {
        assert!(serde_json::from_str::<ImportScope>("\"appearance_and_everything\"").is_err());
        let base = baseline();
        let mut invalid = base.clone();
        invalid.hotkeys.edit_mode = invalid.hotkeys.toggle_overlay.clone();
        assert!(merge(&base, invalid.clone(), ImportScope::Hotkeys).is_err());
        assert!(merge(&base, invalid, ImportScope::Full).is_err());
    }

    #[test]
    fn scoped_imports_preserve_unrelated_editor_settings() {
        let base = baseline();
        let mut candidate = base.clone();
        candidate.style.opacity = 100;
        candidate.units.speed = "mph".into();
        candidate.timing.mini_sectors = 80;
        candidate.hotkeys.toggle_overlay = "Ctrl+F8".into();
        candidate
            .presets
            .custom
            .push(overlay_renderer::config::CustomPresetConfig::default());
        candidate.window.x = 900;
        let appearance = merge(&base, candidate.clone(), ImportScope::Appearance).unwrap();
        assert_eq!(appearance.style.opacity, 100);
        assert_eq!(appearance.units.speed, "mph");
        assert_eq!(appearance.window, base.window);
        assert_eq!(appearance.hotkeys, base.hotkeys);
        assert_eq!(appearance.timing, base.timing);
        let timing = merge(&base, candidate.clone(), ImportScope::Timing).unwrap();
        assert_eq!(timing.timing.mini_sectors, 80);
        assert_eq!(timing.style, base.style);
        let hotkeys = merge(&base, candidate.clone(), ImportScope::Hotkeys).unwrap();
        assert_eq!(hotkeys.hotkeys.toggle_overlay, "Ctrl+F8");
        assert_eq!(hotkeys.presets, base.presets);
        let presets = merge(&base, candidate, ImportScope::Presets).unwrap();
        assert_eq!(presets.presets.custom.len(), 1);
        assert_eq!(presets.window, base.window);
        assert_eq!(base.presets.custom.len(), 0);
    }
    #[test]
    fn surface_imports_preserve_custom_sampling_and_widget_style() {
        let mut base = baseline();
        base.performance.mode = "custom".into();
        base.window.sample_ms = 37;
        base.overlays[0].window.sample_ms = 41;
        let mut candidate = base.clone();
        candidate.window.x = -1920;
        candidate.overlays[0].window.x = -1800;
        candidate.overlays[0].window.sample_ms = 5;
        candidate.style.opacity = 50;
        let result = merge(&base, candidate, ImportScope::Surfaces).unwrap();
        assert_eq!(result.window.x, -1920);
        assert_eq!(result.window.sample_ms, 37);
        assert_eq!(result.overlays[0].window.x, -1800);
        assert_eq!(result.overlays[0].window.sample_ms, 41);
        assert_eq!(result.style, base.style);
        assert_eq!(result.presets, base.presets);
    }
}

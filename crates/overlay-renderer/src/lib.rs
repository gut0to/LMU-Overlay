mod platform;

pub mod config;
pub mod widgets;

pub use platform::{HostRuntimeStats, OverlayError, SharedRuntimeView, TelemetryOverlay};
pub use widgets::{widget_catalog, WidgetDefinition, WIDGET_CATALOG};

mod platform;

pub mod config;
pub mod widgets;

pub use platform::{OverlayError, TelemetryOverlay};
pub use widgets::{widget_catalog, WidgetDefinition, WIDGET_CATALOG};

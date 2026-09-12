use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct WidgetDefinition {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub description: &'static str,
    pub data_requirement: &'static str,
}

pub const WIDGET_CATALOG: &[WidgetDefinition] = &[
    WidgetDefinition {
        id: "telemetry",
        name: "Gear and RPM",
        category: "HUD",
        description: "Gear, speed and engine speed.",
        data_requirement: "fast telemetry",
    },
    WidgetDefinition {
        id: "speed",
        name: "Speed",
        category: "HUD",
        description: "Dedicated speed readout with selectable units.",
        data_requirement: "fast telemetry",
    },
    WidgetDefinition {
        id: "rpm",
        name: "RPM and shift lights",
        category: "HUD",
        description: "Engine-speed bar and shift warning.",
        data_requirement: "fast telemetry",
    },
    WidgetDefinition {
        id: "inputs",
        name: "Pedals",
        category: "HUD",
        description: "Throttle, brake and clutch bars.",
        data_requirement: "fast telemetry",
    },
    WidgetDefinition {
        id: "steering",
        name: "Steering",
        category: "HUD",
        description: "Steering position indicator.",
        data_requirement: "fast telemetry",
    },
    WidgetDefinition {
        id: "input_history",
        name: "Input trace",
        category: "Analysis",
        description: "Bounded trace of driver inputs.",
        data_requirement: "fast telemetry history",
    },
    WidgetDefinition {
        id: "lap_timing",
        name: "Lap timing",
        category: "Timing",
        description: "Current, last, session and personal-best timing.",
        data_requirement: "lap engine",
    },
    WidgetDefinition {
        id: "timing",
        name: "Live delta",
        category: "Timing",
        description: "Reference delta and predicted lap.",
        data_requirement: "lap engine",
    },
    WidgetDefinition {
        id: "sectors",
        name: "Sectors",
        category: "Timing",
        description: "S1, S2 and S3 when officially exposed.",
        data_requirement: "official scoring",
    },
    WidgetDefinition {
        id: "mini_sectors",
        name: "Mini sectors",
        category: "Timing",
        description: "Progress slices against the selected reference.",
        data_requirement: "lap engine",
    },
    WidgetDefinition {
        id: "lap_history",
        name: "Lap history",
        category: "Timing",
        description: "Recent valid and invalid laps.",
        data_requirement: "lap engine",
    },
    WidgetDefinition {
        id: "position",
        name: "Position",
        category: "Race",
        description: "Player position and lap.",
        data_requirement: "official scoring",
    },
    WidgetDefinition {
        id: "relative",
        name: "Relative",
        category: "Race",
        description: "Nearby cars from official scoring data.",
        data_requirement: "official scoring",
    },
    WidgetDefinition {
        id: "standings",
        name: "Standings",
        category: "Race",
        description: "Configurable classification table.",
        data_requirement: "official scoring",
    },
    WidgetDefinition {
        id: "flags",
        name: "Flags",
        category: "Race",
        description: "Session and local flag status.",
        data_requirement: "official scoring",
    },
    WidgetDefinition {
        id: "fuel",
        name: "Fuel",
        category: "Fuel",
        description: "Fuel quantity, consumption and remaining estimate.",
        data_requirement: "official vehicle data",
    },
    WidgetDefinition {
        id: "tyres",
        name: "Tyres",
        category: "Tyres",
        description: "Pressure, temperature, wear and grip by wheel.",
        data_requirement: "official wheel data",
    },
    WidgetDefinition {
        id: "brakes",
        name: "Brakes",
        category: "Brakes",
        description: "Brake temperature, pressure and bias.",
        data_requirement: "official wheel data",
    },
    WidgetDefinition {
        id: "electronics",
        name: "TC and ABS",
        category: "Electronics",
        description: "Electronic-aid state and settings.",
        data_requirement: "official vehicle data",
    },
    WidgetDefinition {
        id: "energy",
        name: "Hybrid energy",
        category: "Energy",
        description: "Battery, virtual energy and hybrid state.",
        data_requirement: "official vehicle data",
    },
    WidgetDefinition {
        id: "engine",
        name: "Engine",
        category: "Car",
        description: "Temperatures, boost and limiter state.",
        data_requirement: "official vehicle data",
    },
    WidgetDefinition {
        id: "damage",
        name: "Damage",
        category: "Car",
        description: "Damage, tyre and impact warnings.",
        data_requirement: "official vehicle data",
    },
    WidgetDefinition {
        id: "weather",
        name: "Weather",
        category: "Utility",
        description: "Track and ambient conditions.",
        data_requirement: "official session data",
    },
    WidgetDefinition {
        id: "coaching",
        name: "Coaching",
        category: "Analysis",
        description: "Reference-lap braking, throttle, speed and gear hints.",
        data_requirement: "lap engine",
    },
    WidgetDefinition {
        id: "performance",
        name: "Performance monitor",
        category: "Utility",
        description: "Overlay acquisition and rendering health.",
        data_requirement: "runtime",
    },
];

pub fn widget_catalog() -> &'static [WidgetDefinition] {
    WIDGET_CATALOG
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn catalog_has_unique_ids_and_core_categories() {
        let ids = WIDGET_CATALOG
            .iter()
            .map(|widget| widget.id)
            .collect::<BTreeSet<_>>();

        assert_eq!(ids.len(), WIDGET_CATALOG.len());
        assert!(WIDGET_CATALOG.iter().any(|widget| widget.id == "fuel"));
        assert!(WIDGET_CATALOG
            .iter()
            .any(|widget| widget.category == "Timing"));
        assert!(WIDGET_CATALOG
            .iter()
            .any(|widget| widget.category == "Race"));
    }
}

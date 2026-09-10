use lmu_telemetry::Gear;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TelemetrySnapshot {
    pub throttle: f64,
    pub brake: f64,
    pub clutch: f64,
    pub steering: f64,
    pub rpm: f64,
    pub gear: Gear,
    pub speed_kph: f64,
    pub lap_number: i32,
    pub sector: i32,
}


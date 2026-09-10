use lmu_telemetry::Gear;
use lmu_telemetry::TelemetrySample;

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

impl From<TelemetrySample> for TelemetrySnapshot {
    fn from(sample: TelemetrySample) -> Self {
        let sample = sample.sanitized();

        Self {
            throttle: sample.throttle,
            brake: sample.brake,
            clutch: sample.clutch,
            steering: sample.steering,
            rpm: sample.rpm,
            gear: sample.gear,
            speed_kph: sample.speed_kph(),
            lap_number: sample.lap_number,
            sector: sample.sector,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_sanitized_snapshot_from_sample() {
        let snapshot = TelemetrySnapshot::from(TelemetrySample {
            timestamp_seconds: 0.0,
            speed_mps: 10.0,
            rpm: -5.0,
            gear: Gear::Forward(3),
            throttle: 2.0,
            brake: 0.25,
            clutch: 0.0,
            steering: -2.0,
            lap_number: 4,
            lap_start_seconds: 0.0,
            sector: 1,
        });

        assert_eq!(snapshot.speed_kph, 36.0);
        assert_eq!(snapshot.rpm, 0.0);
        assert_eq!(snapshot.throttle, 1.0);
        assert_eq!(snapshot.steering, -1.0);
    }
}

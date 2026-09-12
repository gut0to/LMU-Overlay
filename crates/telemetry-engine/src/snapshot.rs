use lmu_telemetry::{GamePhase, Gear, SessionKind, TelemetrySample};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TelemetrySnapshot {
    pub throttle: f64,
    pub brake: f64,
    pub clutch: f64,
    pub steering: f64,
    pub rpm: f64,
    pub gear: Gear,
    pub speed_kph: f64,
    pub lap_distance_m: Option<f64>,
    pub track_length_m: Option<f64>,
    pub lap_time_seconds: Option<f64>,
    pub lap_progress: Option<f64>,
    pub lap_number: i32,
    pub sector: i32,
    pub session_elapsed_seconds: f64,
    pub session_kind: SessionKind,
    pub game_phase: GamePhase,
    pub in_pits: bool,
    pub in_garage: bool,
    pub lap_invalidated: Option<bool>,
    pub player_slot_id: i32,
    pub delta_seconds: Option<f64>,
    pub predicted_lap_seconds: Option<f64>,
    pub session_best_seconds: Option<f64>,
    pub personal_best_seconds: Option<f64>,
    pub reference_lap_seconds: Option<f64>,
    pub mini_sector_index: Option<u16>,
    pub mini_sector_delta_seconds: Option<f64>,
    pub brake_hint_meters: Option<f64>,
    pub throttle_hint_meters: Option<f64>,
    pub speed_hint_kph: Option<f64>,
    pub reference_gear: Option<i32>,
    pub reference_steering: Option<f64>,
    pub reference_throttle: Option<f64>,
    pub reference_brake: Option<f64>,
    pub reference_speed_kph: Option<f64>,
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
            lap_distance_m: sample.lap_distance_m,
            track_length_m: sample.track_length_m,
            lap_time_seconds: sample.lap_time_seconds(),
            lap_progress: sample.lap_progress(),
            lap_number: sample.lap_number,
            sector: sample.sector,
            session_elapsed_seconds: sample.timestamp_seconds,
            session_kind: sample.metadata.session_kind,
            game_phase: sample.metadata.game_phase,
            in_pits: sample.metadata.in_pits,
            in_garage: sample.metadata.in_garage,
            lap_invalidated: sample.metadata.lap_invalidated,
            player_slot_id: sample.metadata.player_slot_id,
            delta_seconds: None,
            predicted_lap_seconds: None,
            session_best_seconds: None,
            personal_best_seconds: None,
            reference_lap_seconds: None,
            mini_sector_index: None,
            mini_sector_delta_seconds: None,
            brake_hint_meters: None,
            throttle_hint_meters: None,
            speed_hint_kph: None,
            reference_gear: None,
            reference_steering: None,
            reference_throttle: None,
            reference_brake: None,
            reference_speed_kph: None,
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
            lap_distance_m: Some(100.0),
            track_length_m: Some(1_000.0),
            lap_number: 4,
            lap_start_seconds: 0.0,
            sector: 1,
            metadata: lmu_telemetry::TelemetryMetadata::default(),
        });

        assert_eq!(snapshot.speed_kph, 36.0);
        assert_eq!(snapshot.rpm, 0.0);
        assert_eq!(snapshot.throttle, 1.0);
        assert_eq!(snapshot.steering, -1.0);
        assert_eq!(snapshot.lap_distance_m, Some(100.0));
        assert_eq!(snapshot.track_length_m, Some(1_000.0));
        assert_eq!(snapshot.lap_time_seconds, Some(0.0));
        assert_eq!(snapshot.lap_progress, Some(0.1));
        assert_eq!(snapshot.delta_seconds, None);
        assert_eq!(snapshot.session_kind, SessionKind::Unknown(-1));
        assert_eq!(snapshot.session_elapsed_seconds, 0.0);
        assert_eq!(snapshot.lap_invalidated, None);
        assert_eq!(snapshot.player_slot_id, -1);
    }
}

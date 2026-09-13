use std::collections::VecDeque;

use lmu_telemetry::TelemetrySample;

const MAX_HISTORY: usize = 8;
const MIN_VALID_SAMPLES: usize = 2;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct FuelAnalysis {
    pub current_liters: Option<f64>,
    pub capacity_liters: Option<f64>,
    pub last_lap_used: Option<f64>,
    pub average_lap_used: Option<f64>,
    pub estimated_laps_remaining: Option<f64>,
}

#[derive(Debug, Default)]
pub struct FuelEngine {
    session_key: Option<(String, String)>,
    lap_number: Option<i32>,
    lap_start_fuel: Option<f64>,
    history: VecDeque<f64>,
    last_lap_used: Option<f64>,
}

impl FuelEngine {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn update(&mut self, sample: &TelemetrySample) -> FuelAnalysis {
        let fuel = sample
            .vehicle
            .fuel_liters
            .filter(|value| value.is_finite() && *value >= 0.0);
        let capacity = sample
            .vehicle
            .fuel_capacity_liters
            .filter(|value| value.is_finite() && *value > 0.0);
        let session_key = (
            sample.metadata.track_name.clone().unwrap_or_default(),
            sample.metadata.vehicle_name.clone().unwrap_or_default(),
        );
        if self.session_key.as_ref() != Some(&session_key) {
            self.reset();
            self.session_key = Some(session_key);
        }

        let lap = sample.lap_number;
        if self.lap_number.is_none() {
            self.lap_number = Some(lap);
            self.lap_start_fuel = fuel;
        } else if self.lap_number != Some(lap) {
            if let (Some(start), Some(end)) = (self.lap_start_fuel, fuel) {
                let used = start - end;
                if sample.metadata.lap_invalidated != Some(true)
                    && !sample.metadata.in_pits
                    && used > 0.0
                    && used < capacity.unwrap_or(f64::INFINITY) * 0.75
                {
                    self.history.push_back(used);
                    self.history.truncate(MAX_HISTORY);
                    self.last_lap_used = Some(used);
                }
            }
            self.lap_number = Some(lap);
            self.lap_start_fuel = fuel;
        }

        let average = (self.history.len() >= MIN_VALID_SAMPLES)
            .then(|| self.history.iter().sum::<f64>() / self.history.len() as f64);
        FuelAnalysis {
            current_liters: fuel,
            capacity_liters: capacity,
            last_lap_used: self.last_lap_used,
            average_lap_used: average,
            estimated_laps_remaining: average.and_then(|used| fuel.map(|current| current / used)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lmu_telemetry::{Gear, TelemetryMetadata, VehicleSystems};

    #[test]
    fn estimates_only_after_two_valid_laps() {
        let mut engine = FuelEngine::default();
        let mut sample = sample(1, 30.0);
        assert_eq!(engine.update(&sample).average_lap_used, None);
        sample.lap_number = 2;
        sample.vehicle.fuel_liters = Some(27.0);
        engine.update(&sample);
        sample.lap_number = 3;
        sample.vehicle.fuel_liters = Some(24.0);
        let analysis = engine.update(&sample);
        assert_eq!(analysis.average_lap_used, Some(3.0));
        assert_eq!(analysis.estimated_laps_remaining, Some(8.0));
    }

    #[test]
    fn ignores_invalid_lap() {
        let mut engine = FuelEngine::default();
        let mut sample = sample(1, 30.0);
        engine.update(&sample);
        sample.lap_number = 2;
        sample.metadata.lap_invalidated = Some(true);
        sample.vehicle.fuel_liters = Some(27.0);
        assert_eq!(engine.update(&sample).last_lap_used, None);
    }

    fn sample(lap: i32, fuel: f64) -> TelemetrySample {
        TelemetrySample {
            timestamp_seconds: 0.0,
            speed_mps: 0.0,
            rpm: 0.0,
            gear: Gear::Neutral,
            throttle: 0.0,
            brake: 0.0,
            clutch: 0.0,
            steering: 0.0,
            lap_distance_m: None,
            track_length_m: None,
            lap_number: lap,
            lap_start_seconds: 0.0,
            sector: 0,
            sector_times: Default::default(),
            vehicle: VehicleSystems {
                fuel_liters: Some(fuel),
                fuel_capacity_liters: Some(100.0),
                ..Default::default()
            },
            wheels: Default::default(),
            session: Default::default(),
            metadata: TelemetryMetadata {
                track_name: Some("track".into()),
                vehicle_name: Some("car".into()),
                ..Default::default()
            },
            field: std::sync::Arc::from(Vec::new()),
        }
    }
}

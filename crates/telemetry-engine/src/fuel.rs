use std::collections::VecDeque;

use lmu_telemetry::TelemetrySample;

const MAX_HISTORY: usize = 8;
const MIN_VALID_SAMPLES: usize = 2;
const REFUEL_NOISE_TOLERANCE_LITERS: f64 = 0.25;

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
    session_key: Option<(String, String, i32, lmu_telemetry::SessionKind)>,
    lap_number: Option<i32>,
    lap_start_fuel: Option<f64>,
    last_fuel: Option<f64>,
    last_timestamp: Option<f64>,
    lap_invalidated: bool,
    entered_pits: bool,
    entered_garage: bool,
    refueled: bool,
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
            sample.metadata.player_slot_id,
            sample.metadata.session_kind,
        );
        if self.session_key.as_ref() != Some(&session_key)
            || self
                .last_timestamp
                .is_some_and(|previous| sample.timestamp_seconds + 1.0 < previous)
            || self
                .lap_number
                .is_some_and(|previous| sample.lap_number < previous)
        {
            self.reset();
            self.session_key = Some(session_key);
        }

        let lap = sample.lap_number;
        if self.lap_number.is_none() {
            self.lap_number = Some(lap);
            self.lap_start_fuel = fuel;
        } else if self.lap_number != Some(lap) {
            let transition_refueled =
                self.last_fuel.zip(fuel).is_some_and(|(previous, current)| {
                    current - previous > REFUEL_NOISE_TOLERANCE_LITERS
                });
            if let (Some(start), Some(end)) = (self.lap_start_fuel, fuel) {
                let used = start - end;
                if !(self.lap_invalidated || sample.metadata.lap_invalidated == Some(true))
                    && !(self.entered_pits || sample.metadata.in_pits)
                    && !(self.entered_garage || sample.metadata.in_garage)
                    && !(self.refueled || transition_refueled)
                    && used > 0.0
                    && used < capacity.unwrap_or(f64::INFINITY) * 0.75
                {
                    self.history.push_back(used);
                    while self.history.len() > MAX_HISTORY {
                        self.history.pop_front();
                    }
                    self.last_lap_used = Some(used);
                }
            }
            self.lap_number = Some(lap);
            self.lap_start_fuel = fuel;
            self.lap_invalidated = false;
            self.entered_pits = false;
            self.entered_garage = false;
            self.refueled = false;
        }

        self.lap_invalidated |= sample.metadata.lap_invalidated == Some(true);
        self.entered_pits |= sample.metadata.in_pits;
        self.entered_garage |= sample.metadata.in_garage;
        if let (Some(previous), Some(current)) = (self.last_fuel, fuel) {
            self.refueled |= current - previous > REFUEL_NOISE_TOLERANCE_LITERS;
        }
        self.last_fuel = fuel.or(self.last_fuel);
        self.last_timestamp = Some(sample.timestamp_seconds);

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

    #[test]
    fn rolling_average_uses_recent_laps() {
        let mut engine = FuelEngine::default();
        let mut sample = sample(1, 100.0);
        engine.update(&sample);

        let mut fuel = 100.0;
        for lap in 2..=10 {
            sample.lap_number = lap;
            fuel -= f64::from(lap - 1);
            sample.vehicle.fuel_liters = Some(fuel);
            engine.update(&sample);
        }

        sample.lap_number = 11;
        sample.vehicle.fuel_liters = Some(fuel - 10.0);
        let analysis = engine.update(&sample);

        // The first sample (1 L) has been evicted; the eight newest are
        // 2..9 L plus the just-finalized 10 L sample.
        assert_eq!(analysis.average_lap_used, Some(6.5));
    }

    #[test]
    fn invalidation_and_pits_anywhere_in_lap_exclude_the_lap() {
        let mut engine = FuelEngine::default();
        let mut sample = sample(1, 30.0);
        engine.update(&sample);
        sample.vehicle.fuel_liters = Some(29.0);
        sample.metadata.lap_invalidated = Some(true);
        engine.update(&sample);
        sample.lap_number = 2;
        sample.metadata.lap_invalidated = Some(false);
        sample.vehicle.fuel_liters = Some(28.0);
        assert_eq!(engine.update(&sample).last_lap_used, None);

        sample.metadata.in_pits = true;
        sample.vehicle.fuel_liters = Some(27.0);
        engine.update(&sample);
        sample.lap_number = 3;
        sample.metadata.in_pits = false;
        sample.vehicle.fuel_liters = Some(26.0);
        assert_eq!(engine.update(&sample).last_lap_used, None);
    }

    #[test]
    fn garage_and_refuel_noise_do_not_contaminate_valid_history() {
        let mut engine = FuelEngine::default();
        let mut garage_sample = sample(1, 30.0);
        engine.update(&garage_sample);
        garage_sample.metadata.in_garage = true;
        garage_sample.vehicle.fuel_liters = Some(29.0);
        engine.update(&garage_sample);
        garage_sample.lap_number = 2;
        garage_sample.metadata.in_garage = false;
        garage_sample.vehicle.fuel_liters = Some(28.0);
        assert_eq!(engine.update(&garage_sample).last_lap_used, None);

        let mut refuel_sample = sample(1, 30.0);
        let mut engine = FuelEngine::default();
        engine.update(&refuel_sample);
        refuel_sample.vehicle.fuel_liters = Some(30.2);
        engine.update(&refuel_sample);
        refuel_sample.lap_number = 2;
        refuel_sample.vehicle.fuel_liters = Some(29.0);
        assert_eq!(engine.update(&refuel_sample).last_lap_used, Some(1.0));
    }

    #[test]
    fn session_marker_changes_reset_previous_average() {
        let mut engine = FuelEngine::default();
        let mut sample = sample(1, 30.0);
        engine.update(&sample);
        sample.lap_number = 2;
        sample.vehicle.fuel_liters = Some(27.0);
        engine.update(&sample);
        sample.lap_number = 3;
        sample.vehicle.fuel_liters = Some(24.0);
        assert_eq!(engine.update(&sample).average_lap_used, Some(3.0));

        sample.metadata.player_slot_id = 99;
        sample.lap_number = 1;
        sample.vehicle.fuel_liters = Some(50.0);
        assert_eq!(engine.update(&sample).average_lap_used, None);
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

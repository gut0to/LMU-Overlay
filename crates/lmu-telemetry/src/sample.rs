use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gear {
    Reverse,
    Neutral,
    Forward(u8),
    Unknown(i32),
}

impl From<i32> for Gear {
    fn from(value: i32) -> Self {
        match value {
            -1 => Self::Reverse,
            0 => Self::Neutral,
            1..=99 => Self::Forward(value as u8),
            other => Self::Unknown(other),
        }
    }
}

impl fmt::Display for Gear {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Gear::Reverse => f.write_str("R"),
            Gear::Neutral => f.write_str("N"),
            Gear::Forward(value) => write!(f, "{value}"),
            Gear::Unknown(value) => write!(f, "?({value})"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionKind {
    TestDay,
    Practice,
    Qualifying,
    Warmup,
    Race,
    Unknown(i32),
}

impl From<i32> for SessionKind {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::TestDay,
            1..=4 => Self::Practice,
            5..=8 => Self::Qualifying,
            9 => Self::Warmup,
            10..=13 => Self::Race,
            other => Self::Unknown(other),
        }
    }
}

impl fmt::Display for SessionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TestDay => f.write_str("test day"),
            Self::Practice => f.write_str("practice"),
            Self::Qualifying => f.write_str("qualifying"),
            Self::Warmup => f.write_str("warmup"),
            Self::Race => f.write_str("race"),
            Self::Unknown(value) => write!(f, "unknown({value})"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GamePhase {
    BeforeSession,
    Reconnaissance,
    GridWalk,
    FormationLap,
    StartingLights,
    GreenFlag,
    FullCourseYellow,
    SessionStopped,
    SessionOver,
    Unknown(u8),
}

impl From<u8> for GamePhase {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::BeforeSession,
            1 => Self::Reconnaissance,
            2 => Self::GridWalk,
            3 => Self::FormationLap,
            4 => Self::StartingLights,
            5 => Self::GreenFlag,
            6 => Self::FullCourseYellow,
            7 => Self::SessionStopped,
            8 => Self::SessionOver,
            other => Self::Unknown(other),
        }
    }
}

impl fmt::Display for GamePhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BeforeSession => f.write_str("before session"),
            Self::Reconnaissance => f.write_str("reconnaissance"),
            Self::GridWalk => f.write_str("grid walk"),
            Self::FormationLap => f.write_str("formation lap"),
            Self::StartingLights => f.write_str("starting lights"),
            Self::GreenFlag => f.write_str("green flag"),
            Self::FullCourseYellow => f.write_str("full course yellow"),
            Self::SessionStopped => f.write_str("session stopped"),
            Self::SessionOver => f.write_str("session over"),
            Self::Unknown(value) => write!(f, "unknown({value})"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelemetryMetadata {
    pub track_name: Option<String>,
    pub track_layout: Option<String>,
    pub vehicle_name: Option<String>,
    pub vehicle_class: Option<String>,
    pub session_kind: SessionKind,
    pub game_phase: GamePhase,
    pub in_pits: bool,
    pub in_garage: bool,
    pub lap_invalidated: Option<bool>,
    pub player_slot_id: i32,
}

impl Default for TelemetryMetadata {
    fn default() -> Self {
        Self {
            track_name: None,
            track_layout: None,
            vehicle_name: None,
            vehicle_class: None,
            session_kind: SessionKind::Unknown(-1),
            game_phase: GamePhase::Unknown(u8::MAX),
            in_pits: false,
            in_garage: false,
            lap_invalidated: None,
            player_slot_id: -1,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TelemetrySample {
    pub timestamp_seconds: f64,
    pub speed_mps: f64,
    pub rpm: f64,
    pub gear: Gear,
    pub throttle: f64,
    pub brake: f64,
    pub clutch: f64,
    pub steering: f64,
    pub lap_distance_m: Option<f64>,
    pub track_length_m: Option<f64>,
    pub lap_number: i32,
    pub lap_start_seconds: f64,
    pub sector: i32,
    pub metadata: TelemetryMetadata,
}

impl TelemetrySample {
    pub fn sanitized(mut self) -> Self {
        self.speed_mps = non_negative_finite(self.speed_mps);
        self.rpm = non_negative_finite(self.rpm);
        self.throttle = normalized_input(self.throttle);
        self.brake = normalized_input(self.brake);
        self.clutch = normalized_input(self.clutch);
        self.steering = self.steering.clamp(-1.0, 1.0);
        self.lap_distance_m = self.lap_distance_m.map(non_negative_finite);
        self.track_length_m = self.track_length_m.map(non_negative_finite);
        self
    }

    pub fn speed_kph(&self) -> f64 {
        self.speed_mps * 3.6
    }

    pub fn lap_time_seconds(&self) -> Option<f64> {
        let lap_time = self.timestamp_seconds - self.lap_start_seconds;
        lap_time
            .is_finite()
            .then_some(lap_time)
            .filter(|time| *time >= 0.0)
    }

    pub fn lap_progress(&self) -> Option<f64> {
        let distance = self.lap_distance_m?;
        let length = self.track_length_m?;

        (length > 0.0).then_some((distance / length).clamp(0.0, 1.0))
    }
}

pub fn format_sample_line(sample: &TelemetrySample) -> String {
    format!(
        "speed={:.1} km/h gear={} throttle={:.0}% brake={:.0}% rpm={:.0} lap={} sector={}",
        sample.speed_kph(),
        sample.gear,
        sample.throttle * 100.0,
        sample.brake * 100.0,
        sample.rpm,
        sample.lap_number,
        sample.sector
    )
}

fn normalized_input(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn non_negative_finite(value: f64) -> f64 {
    if value.is_finite() && value >= 0.0 {
        value
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_gear_values() {
        assert_eq!(Gear::from(-1), Gear::Reverse);
        assert_eq!(Gear::from(0), Gear::Neutral);
        assert_eq!(Gear::from(6), Gear::Forward(6));
        assert_eq!(Gear::from(123), Gear::Unknown(123));
    }

    #[test]
    fn formats_sample_line() {
        let sample = TelemetrySample {
            timestamp_seconds: 42.0,
            speed_mps: 50.0,
            rpm: 7123.0,
            gear: Gear::Forward(4),
            throttle: 0.5,
            brake: 0.25,
            clutch: 0.0,
            steering: -0.1,
            lap_distance_m: Some(1_000.0),
            track_length_m: Some(5_000.0),
            lap_number: 3,
            lap_start_seconds: 30.0,
            sector: 2,
            metadata: TelemetryMetadata::default(),
        };

        assert_eq!(
            format_sample_line(&sample),
            "speed=180.0 km/h gear=4 throttle=50% brake=25% rpm=7123 lap=3 sector=2"
        );
    }

    #[test]
    fn sanitizes_sample_values_for_ui_consumers() {
        let sample = TelemetrySample {
            timestamp_seconds: 12.0,
            speed_mps: f64::NAN,
            rpm: -1.0,
            gear: Gear::Neutral,
            throttle: 1.5,
            brake: -0.5,
            clutch: f64::INFINITY,
            steering: 3.0,
            lap_distance_m: Some(-10.0),
            track_length_m: Some(f64::NAN),
            lap_number: 1,
            lap_start_seconds: 10.0,
            sector: 0,
            metadata: TelemetryMetadata::default(),
        }
        .sanitized();

        assert_eq!(sample.speed_mps, 0.0);
        assert_eq!(sample.rpm, 0.0);
        assert_eq!(sample.throttle, 1.0);
        assert_eq!(sample.brake, 0.0);
        assert_eq!(sample.clutch, 0.0);
        assert_eq!(sample.steering, 1.0);
        assert_eq!(sample.lap_distance_m, Some(0.0));
        assert_eq!(sample.track_length_m, Some(0.0));
        assert_eq!(sample.lap_time_seconds(), Some(2.0));
    }

    #[test]
    fn computes_lap_progress_when_track_length_is_available() {
        let sample = TelemetrySample {
            timestamp_seconds: 0.0,
            speed_mps: 0.0,
            rpm: 0.0,
            gear: Gear::Neutral,
            throttle: 0.0,
            brake: 0.0,
            clutch: 0.0,
            steering: 0.0,
            lap_distance_m: Some(2_500.0),
            track_length_m: Some(5_000.0),
            lap_number: 1,
            lap_start_seconds: 0.0,
            sector: 0,
            metadata: TelemetryMetadata::default(),
        };

        assert_eq!(sample.lap_progress(), Some(0.5));
    }

    #[test]
    fn maps_session_and_game_phase_values() {
        assert_eq!(SessionKind::from(2), SessionKind::Practice);
        assert_eq!(SessionKind::from(7), SessionKind::Qualifying);
        assert_eq!(SessionKind::from(12), SessionKind::Race);
        assert_eq!(GamePhase::from(5), GamePhase::GreenFlag);
        assert_eq!(GamePhase::from(8), GamePhase::SessionOver);
    }
}

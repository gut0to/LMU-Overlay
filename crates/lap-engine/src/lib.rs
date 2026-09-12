use serde::{Deserialize, Serialize};
use telemetry_engine::TelemetrySnapshot;

const NORMALIZED_REFERENCE_POINTS: usize = 2_001;
const MAX_MINI_SECTORS: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReferenceMode {
    PersonalBest,
    SessionBest,
    BestValidLap,
    LastLap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PerformanceMode {
    Eco,
    Normal,
    HighRefresh,
}

impl PerformanceMode {
    pub fn telemetry_hz(self) -> u64 {
        match self {
            Self::Eco => 50,
            Self::Normal | Self::HighRefresh => 100,
        }
    }

    pub fn render_fps(self) -> u64 {
        match self {
            Self::Eco => 30,
            Self::Normal => 60,
            Self::HighRefresh => 120,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LapEngineConfig {
    pub reference_mode: ReferenceMode,
    pub mini_sectors: u16,
    pub brake_threshold: f64,
    pub throttle_threshold: f64,
    pub event_match_tolerance_m: f64,
    pub min_reference_points: usize,
}

impl Default for LapEngineConfig {
    fn default() -> Self {
        Self {
            reference_mode: ReferenceMode::PersonalBest,
            mini_sectors: 40,
            brake_threshold: 0.10,
            throttle_threshold: 0.10,
            event_match_tolerance_m: 80.0,
            min_reference_points: 20,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ReferencePoint {
    pub progress: f64,
    pub time_seconds: f64,
    pub throttle: f64,
    pub brake: f64,
    pub speed_kph: f64,
    pub gear: i32,
    pub steering: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DrivingEventKind {
    BrakeStart,
    BrakeRelease,
    ThrottleStart,
    FullThrottle,
    GearUp,
    GearDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DrivingEvent {
    pub kind: DrivingEventKind,
    pub progress: f64,
    pub distance_m: Option<f64>,
    pub time_seconds: f64,
    pub speed_kph: f64,
    pub gear: i32,
    pub input_value: f64,
}

impl ReferencePoint {
    pub fn from_snapshot(snapshot: TelemetrySnapshot) -> Option<Self> {
        Some(Self {
            progress: snapshot.lap_progress?,
            time_seconds: snapshot.lap_time_seconds?,
            throttle: snapshot.throttle,
            brake: snapshot.brake,
            speed_kph: snapshot.speed_kph,
            gear: gear_number(snapshot.gear),
            steering: snapshot.steering,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReferenceLap {
    pub total_time_seconds: f64,
    pub points: Vec<ReferencePoint>,
    pub events: Vec<DrivingEvent>,
}

impl ReferenceLap {
    pub fn new(total_time_seconds: f64, mut points: Vec<ReferencePoint>) -> Option<Self> {
        if !total_time_seconds.is_finite() || total_time_seconds <= 0.0 {
            return None;
        }

        points.retain(|point| {
            point.progress.is_finite()
                && point.time_seconds.is_finite()
                && point.progress >= 0.0
                && point.progress <= 1.0
                && point.time_seconds >= 0.0
        });
        points.sort_by(|a, b| a.progress.total_cmp(&b.progress));
        points.dedup_by(|a, b| (a.progress - b.progress).abs() < f64::EPSILON);

        if points.is_empty() {
            return None;
        }

        let points = normalize_reference_points(&points);

        let events = driving_events_from_points(&points);

        Some(Self {
            total_time_seconds,
            points,
            events,
        })
    }

    pub fn sample_at(&self, progress: f64) -> Option<ReferencePoint> {
        let progress = progress.clamp(0.0, 1.0);
        let max_index = self.points.len().checked_sub(1)?;
        let scaled = progress * max_index as f64;
        let lower = scaled.floor() as usize;
        let upper = scaled.ceil() as usize;
        let before = self.points[lower.min(max_index)];
        let after = self.points[upper.min(max_index)];

        let span = after.progress - before.progress;
        if span <= f64::EPSILON {
            return Some(before);
        }

        let amount = (progress - before.progress) / span;
        Some(ReferencePoint {
            progress,
            time_seconds: lerp(before.time_seconds, after.time_seconds, amount),
            throttle: lerp(before.throttle, after.throttle, amount),
            brake: lerp(before.brake, after.brake, amount),
            speed_kph: lerp(before.speed_kph, after.speed_kph, amount),
            gear: if amount < 0.5 {
                before.gear
            } else {
                after.gear
            },
            steering: lerp(before.steering, after.steering, amount),
        })
    }

    pub fn threshold_crossing_progress(
        &self,
        value: impl Fn(ReferencePoint) -> f64,
        threshold: f64,
    ) -> Option<f64> {
        self.points.windows(2).find_map(|pair| {
            let before = value(pair[0]);
            let after = value(pair[1]);
            if before >= threshold || after < threshold {
                return None;
            }

            let span = after - before;
            if span <= f64::EPSILON {
                return Some(pair[1].progress);
            }

            let amount = (threshold - before) / span;
            Some(lerp(pair[0].progress, pair[1].progress, amount))
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MiniSectorState {
    Gain,
    Neutral,
    Loss,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MiniSectorResult {
    pub index: u16,
    pub current_time_seconds: f64,
    pub reference_time_seconds: f64,
    pub delta_seconds: f64,
    pub state: MiniSectorState,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LapAnalysis {
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
    pub mini_sector_history: Vec<MiniSectorResult>,
    pub reference_throttle: Option<f64>,
    pub reference_brake: Option<f64>,
    pub reference_speed_kph: Option<f64>,
}

impl LapAnalysis {
    pub fn apply_to(self, snapshot: &mut TelemetrySnapshot) {
        snapshot.delta_seconds = self.delta_seconds;
        snapshot.predicted_lap_seconds = self.predicted_lap_seconds;
        snapshot.session_best_seconds = self.session_best_seconds;
        snapshot.personal_best_seconds = self.personal_best_seconds;
        snapshot.reference_lap_seconds = self.reference_lap_seconds;
        snapshot.mini_sector_index = self.mini_sector_index;
        snapshot.mini_sector_delta_seconds = self.mini_sector_delta_seconds;
        snapshot.brake_hint_meters = self.brake_hint_meters;
        snapshot.throttle_hint_meters = self.throttle_hint_meters;
        snapshot.speed_hint_kph = self.speed_hint_kph;
        snapshot.reference_gear = self.reference_gear;
        snapshot.reference_steering = self.reference_steering;
        snapshot.reference_throttle = self.reference_throttle;
        snapshot.reference_brake = self.reference_brake;
        snapshot.reference_speed_kph = self.reference_speed_kph;
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct SessionMarker {
    session_kind: lmu_telemetry::SessionKind,
    game_phase: lmu_telemetry::GamePhase,
    player_slot_id: i32,
    lap_number: i32,
    session_elapsed_seconds: f64,
    lap_time_seconds: Option<f64>,
}

impl SessionMarker {
    fn from_snapshot(snapshot: TelemetrySnapshot) -> Self {
        Self {
            session_kind: snapshot.session_kind,
            game_phase: snapshot.game_phase,
            player_slot_id: snapshot.player_slot_id,
            lap_number: snapshot.lap_number,
            session_elapsed_seconds: snapshot.session_elapsed_seconds,
            lap_time_seconds: snapshot.lap_time_seconds,
        }
    }

    fn starts_new_session(self, previous: Self) -> bool {
        self.session_kind != previous.session_kind
            || self.player_slot_id != previous.player_slot_id
            || self.lap_number < previous.lap_number
            || self.session_elapsed_seconds + 1.0 < previous.session_elapsed_seconds
            || self
                .lap_time_seconds
                .zip(previous.lap_time_seconds)
                .is_some_and(|(current_lap_time, previous_lap_time)| {
                    self.lap_number == previous.lap_number
                        && current_lap_time + 1.0 < previous_lap_time
                })
            || (previous.game_phase == lmu_telemetry::GamePhase::SessionOver
                && self.game_phase == lmu_telemetry::GamePhase::GreenFlag)
    }
}

#[derive(Debug, Clone)]
pub struct LapEngine {
    config: LapEngineConfig,
    session_marker: Option<SessionMarker>,
    current_lap_number: Option<i32>,
    current_points: Vec<ReferencePoint>,
    current_events: Vec<DrivingEvent>,
    previous_brake: f64,
    previous_throttle: f64,
    previous_gear: i32,
    previous_full_throttle: bool,
    previous_mini_sector_index: Option<u16>,
    previous_boundary_delta: Option<f64>,
    completed_mini_sector_delta: Option<f64>,
    completed_mini_sectors: Vec<MiniSectorResult>,
    current_lap_valid: bool,
    last_lap: Option<ReferenceLap>,
    last_valid_lap: Option<ReferenceLap>,
    best_valid_lap: Option<ReferenceLap>,
    session_best: Option<ReferenceLap>,
    personal_best: Option<ReferenceLap>,
    pending_personal_best: Option<ReferenceLap>,
}

impl LapEngine {
    pub fn new(config: LapEngineConfig) -> Self {
        Self {
            config,
            session_marker: None,
            current_lap_number: None,
            current_points: Vec::with_capacity(2_000),
            current_events: Vec::with_capacity(128),
            previous_brake: 0.0,
            previous_throttle: 0.0,
            previous_gear: 0,
            previous_full_throttle: false,
            previous_mini_sector_index: None,
            previous_boundary_delta: None,
            completed_mini_sector_delta: None,
            completed_mini_sectors: Vec::with_capacity(MAX_MINI_SECTORS),
            current_lap_valid: false,
            last_lap: None,
            last_valid_lap: None,
            best_valid_lap: None,
            session_best: None,
            personal_best: None,
            pending_personal_best: None,
        }
    }

    pub fn with_personal_best(mut self, personal_best: Option<ReferenceLap>) -> Self {
        self.personal_best = personal_best;
        self
    }

    pub fn update(&mut self, snapshot: TelemetrySnapshot) -> LapAnalysis {
        self.reset_if_new_session(snapshot);

        if self.current_lap_number != Some(snapshot.lap_number) {
            self.finish_current_lap(snapshot);
        }

        let sample_is_valid = is_lap_sample_valid(snapshot);
        if !sample_is_valid {
            self.current_lap_valid = false;
        }

        if sample_is_valid {
            self.record_driving_events(snapshot);
        }

        let progress = snapshot.lap_progress;
        let lap_time = snapshot.lap_time_seconds;
        let mini_sector_index = progress.map(|progress| self.mini_sector_index(progress));
        let (
            reference_lap_seconds,
            reference_point,
            delta,
            predicted,
            brake_hint_meters,
            throttle_hint_meters,
            speed_hint_kph,
        ) = {
            let reference = sample_is_valid.then(|| self.selected_reference()).flatten();
            let reference_point = progress
                .and_then(|progress| reference.and_then(|reference| reference.sample_at(progress)));
            let delta = lap_time
                .zip(reference_point)
                .map(|(lap_time, reference_point)| lap_time - reference_point.time_seconds);
            let predicted = reference
                .zip(delta)
                .map(|(reference, delta)| reference.total_time_seconds + delta);
            let brake_hint_meters = self.event_hint_meters(
                reference,
                snapshot.track_length_m,
                DrivingEventKind::BrakeStart,
            );
            let throttle_hint_meters = self.event_hint_meters(
                reference,
                snapshot.track_length_m,
                DrivingEventKind::ThrottleStart,
            );
            let speed_hint_kph = reference_point.map(|point| point.speed_kph - snapshot.speed_kph);

            (
                reference.map(|lap| lap.total_time_seconds),
                reference_point,
                delta,
                predicted,
                brake_hint_meters,
                throttle_hint_meters,
                speed_hint_kph,
            )
        };

        if sample_is_valid {
            self.record_mini_sector_delta(mini_sector_index, lap_time, reference_point, delta);
            self.record_point(snapshot);
        }

        LapAnalysis {
            delta_seconds: delta,
            predicted_lap_seconds: predicted,
            session_best_seconds: self.session_best.as_ref().map(|lap| lap.total_time_seconds),
            personal_best_seconds: self
                .personal_best
                .as_ref()
                .map(|lap| lap.total_time_seconds),
            reference_lap_seconds,
            mini_sector_index,
            mini_sector_delta_seconds: self.completed_mini_sector_delta,
            brake_hint_meters,
            throttle_hint_meters,
            speed_hint_kph,
            reference_gear: reference_point.map(|point| point.gear),
            reference_steering: reference_point.map(|point| point.steering),
            mini_sector_history: self.completed_mini_sectors.clone(),
            reference_throttle: reference_point.map(|point| point.throttle),
            reference_brake: reference_point.map(|point| point.brake),
            reference_speed_kph: reference_point.map(|point| point.speed_kph),
        }
    }

    pub fn session_best(&self) -> Option<&ReferenceLap> {
        self.session_best.as_ref()
    }

    pub fn personal_best(&self) -> Option<&ReferenceLap> {
        self.personal_best.as_ref()
    }

    pub fn take_new_personal_best(&mut self) -> Option<ReferenceLap> {
        self.pending_personal_best.take()
    }

    fn finish_current_lap(&mut self, snapshot: TelemetrySnapshot) {
        if let Some(lap_time) = self.current_points.last().map(|point| point.time_seconds) {
            if self.current_points.len() >= self.config.min_reference_points {
                if let Some(mut lap) = ReferenceLap::new(lap_time, self.current_points.clone()) {
                    lap.events = self.current_events.clone();
                    self.last_lap = Some(lap.clone());
                    if self.current_lap_valid {
                        self.last_valid_lap = Some(lap.clone());
                        if is_better(&self.best_valid_lap, &lap) {
                            self.best_valid_lap = Some(lap.clone());
                        }
                        if is_better(&self.session_best, &lap) {
                            self.session_best = Some(lap.clone());
                        }
                        if is_better(&self.personal_best, &lap) {
                            self.personal_best = Some(lap.clone());
                            self.pending_personal_best = Some(lap);
                        }
                    }
                }
            }
        }

        self.current_lap_number = Some(snapshot.lap_number);
        self.current_points.clear();
        self.current_events.clear();
        self.previous_brake = snapshot.brake;
        self.previous_throttle = snapshot.throttle;
        self.previous_gear = gear_number(snapshot.gear);
        self.previous_full_throttle = snapshot.throttle >= 0.98;
        self.previous_mini_sector_index = None;
        self.previous_boundary_delta = None;
        self.completed_mini_sector_delta = None;
        self.completed_mini_sectors.clear();
        self.current_lap_valid = is_lap_sample_valid(snapshot);
    }

    fn record_point(&mut self, snapshot: TelemetrySnapshot) {
        if let Some(point) = ReferencePoint::from_snapshot(snapshot) {
            if self
                .current_points
                .last()
                .is_none_or(|last| point.progress > last.progress)
            {
                self.current_points.push(point);
            }
        }
    }

    fn record_driving_events(&mut self, snapshot: TelemetrySnapshot) {
        if crossed_up(
            self.previous_brake,
            snapshot.brake,
            self.config.brake_threshold,
        ) {
            self.push_event(snapshot, DrivingEventKind::BrakeStart, snapshot.brake);
        }
        if crossed_down(
            self.previous_brake,
            snapshot.brake,
            self.config.brake_threshold,
        ) {
            self.push_event(snapshot, DrivingEventKind::BrakeRelease, snapshot.brake);
        }
        if crossed_up(
            self.previous_throttle,
            snapshot.throttle,
            self.config.throttle_threshold,
        ) {
            self.push_event(snapshot, DrivingEventKind::ThrottleStart, snapshot.throttle);
        }
        let full_throttle = snapshot.throttle >= 0.98;
        if !self.previous_full_throttle && full_throttle {
            self.push_event(snapshot, DrivingEventKind::FullThrottle, snapshot.throttle);
        }
        let gear = gear_number(snapshot.gear);
        if gear > self.previous_gear {
            self.push_event(snapshot, DrivingEventKind::GearUp, snapshot.throttle);
        } else if gear < self.previous_gear {
            self.push_event(snapshot, DrivingEventKind::GearDown, snapshot.throttle);
        }

        self.previous_brake = snapshot.brake;
        self.previous_throttle = snapshot.throttle;
        self.previous_full_throttle = full_throttle;
        self.previous_gear = gear;
    }

    fn push_event(
        &mut self,
        snapshot: TelemetrySnapshot,
        kind: DrivingEventKind,
        input_value: f64,
    ) {
        let Some(progress) = snapshot.lap_progress else {
            return;
        };
        let event = DrivingEvent {
            kind,
            progress,
            distance_m: snapshot.lap_distance_m,
            time_seconds: snapshot.lap_time_seconds.unwrap_or_default(),
            speed_kph: snapshot.speed_kph,
            gear: gear_number(snapshot.gear),
            input_value,
        };
        self.current_events.push(event);
    }

    fn record_mini_sector_delta(
        &mut self,
        index: Option<u16>,
        lap_time: Option<f64>,
        reference_point: Option<ReferencePoint>,
        delta: Option<f64>,
    ) {
        let Some(index) = index else {
            return;
        };

        let Some(delta) = delta else {
            self.previous_mini_sector_index = Some(index);
            return;
        };

        if self.previous_mini_sector_index.is_none() {
            self.previous_boundary_delta = Some(delta);
        } else if self
            .previous_mini_sector_index
            .is_some_and(|previous| index > previous)
        {
            let own_delta = self
                .previous_boundary_delta
                .map(|previous| delta - previous)
                .unwrap_or(delta);
            self.completed_mini_sector_delta = Some(own_delta);
            self.previous_boundary_delta = Some(delta);
            if self.completed_mini_sectors.len() < MAX_MINI_SECTORS {
                self.completed_mini_sectors.push(MiniSectorResult {
                    index: index.saturating_sub(1),
                    current_time_seconds: lap_time.unwrap_or_default(),
                    reference_time_seconds: reference_point
                        .map(|point| point.time_seconds)
                        .unwrap_or_default(),
                    delta_seconds: own_delta,
                    state: mini_sector_state(own_delta),
                });
            }
        }

        self.previous_mini_sector_index = Some(index);
    }

    fn selected_reference(&self) -> Option<&ReferenceLap> {
        match self.config.reference_mode {
            ReferenceMode::PersonalBest => self
                .personal_best
                .as_ref()
                .or(self.session_best.as_ref())
                .or(self.last_lap.as_ref()),
            ReferenceMode::SessionBest => self
                .session_best
                .as_ref()
                .or(self.personal_best.as_ref())
                .or(self.last_lap.as_ref()),
            ReferenceMode::BestValidLap => self
                .best_valid_lap
                .as_ref()
                .or(self.session_best.as_ref())
                .or(self.personal_best.as_ref()),
            ReferenceMode::LastLap => self
                .last_lap
                .as_ref()
                .or(self.session_best.as_ref())
                .or(self.personal_best.as_ref()),
        }
    }

    fn mini_sector_index(&self, progress: f64) -> u16 {
        let mini_sectors = self.config.mini_sectors.max(1);
        ((progress.clamp(0.0, 0.999_999) * f64::from(mini_sectors)).floor() as u16)
            .min(mini_sectors - 1)
    }

    fn event_hint_meters(
        &self,
        reference: Option<&ReferenceLap>,
        track_length_m: Option<f64>,
        kind: DrivingEventKind,
    ) -> Option<f64> {
        let current_event = self
            .current_events
            .iter()
            .rev()
            .find(|event| event.kind == kind)?;
        let reference_event = matching_reference_event(reference?, current_event)?;
        let track_length_m = track_length_m?;
        let delta_meters = (reference_event.progress - current_event.progress) * track_length_m;
        if delta_meters.abs() > self.config.event_match_tolerance_m {
            return None;
        }

        Some(delta_meters)
    }

    fn reset_if_new_session(&mut self, snapshot: TelemetrySnapshot) {
        let marker = SessionMarker::from_snapshot(snapshot);
        if self
            .session_marker
            .is_some_and(|previous| marker.starts_new_session(previous))
        {
            self.reset_session();
        }
        self.session_marker = Some(marker);
    }

    fn reset_session(&mut self) {
        self.current_lap_number = None;
        self.current_points.clear();
        self.current_events.clear();
        self.previous_brake = 0.0;
        self.previous_throttle = 0.0;
        self.previous_gear = 0;
        self.previous_full_throttle = false;
        self.previous_mini_sector_index = None;
        self.previous_boundary_delta = None;
        self.completed_mini_sector_delta = None;
        self.completed_mini_sectors.clear();
        self.current_lap_valid = false;
        self.last_lap = None;
        self.last_valid_lap = None;
        self.best_valid_lap = None;
        self.session_best = None;
    }
}

impl Default for LapEngine {
    fn default() -> Self {
        Self::new(LapEngineConfig::default())
    }
}

fn normalize_reference_points(points: &[ReferencePoint]) -> Vec<ReferencePoint> {
    let mut normalized = Vec::with_capacity(NORMALIZED_REFERENCE_POINTS);
    for index in 0..NORMALIZED_REFERENCE_POINTS {
        let progress = index as f64 / (NORMALIZED_REFERENCE_POINTS - 1) as f64;
        normalized.push(interpolate_points(points, progress));
    }
    normalized
}

fn driving_events_from_points(points: &[ReferencePoint]) -> Vec<DrivingEvent> {
    let mut events = Vec::new();
    let mut previous_brake = points.first().map(|point| point.brake).unwrap_or_default();
    let mut previous_throttle = points
        .first()
        .map(|point| point.throttle)
        .unwrap_or_default();
    let mut previous_full_throttle = previous_throttle >= 0.98;
    let mut previous_gear = points.first().map(|point| point.gear).unwrap_or_default();

    for point in points.iter().copied().skip(1) {
        if crossed_up(previous_brake, point.brake, 0.10) {
            events.push(event_from_point(
                point,
                DrivingEventKind::BrakeStart,
                point.brake,
            ));
        }
        if crossed_down(previous_brake, point.brake, 0.10) {
            events.push(event_from_point(
                point,
                DrivingEventKind::BrakeRelease,
                point.brake,
            ));
        }
        if crossed_up(previous_throttle, point.throttle, 0.10) {
            events.push(event_from_point(
                point,
                DrivingEventKind::ThrottleStart,
                point.throttle,
            ));
        }
        let full_throttle = point.throttle >= 0.98;
        if !previous_full_throttle && full_throttle {
            events.push(event_from_point(
                point,
                DrivingEventKind::FullThrottle,
                point.throttle,
            ));
        }
        if point.gear > previous_gear {
            events.push(event_from_point(
                point,
                DrivingEventKind::GearUp,
                point.throttle,
            ));
        } else if point.gear < previous_gear {
            events.push(event_from_point(
                point,
                DrivingEventKind::GearDown,
                point.throttle,
            ));
        }

        previous_brake = point.brake;
        previous_throttle = point.throttle;
        previous_full_throttle = full_throttle;
        previous_gear = point.gear;
    }

    events
}

fn event_from_point(
    point: ReferencePoint,
    kind: DrivingEventKind,
    input_value: f64,
) -> DrivingEvent {
    DrivingEvent {
        kind,
        progress: point.progress,
        distance_m: None,
        time_seconds: point.time_seconds,
        speed_kph: point.speed_kph,
        gear: point.gear,
        input_value,
    }
}

fn matching_reference_event<'a>(
    reference: &'a ReferenceLap,
    current: &DrivingEvent,
) -> Option<&'a DrivingEvent> {
    reference
        .events
        .iter()
        .filter(|event| event.kind == current.kind)
        .map(|event| ((event.progress - current.progress).abs(), event))
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, event)| event)
}

fn crossed_up(previous: f64, current: f64, threshold: f64) -> bool {
    previous < threshold && current >= threshold
}

fn crossed_down(previous: f64, current: f64, threshold: f64) -> bool {
    previous >= threshold && current < threshold
}

fn mini_sector_state(delta_seconds: f64) -> MiniSectorState {
    if delta_seconds <= -0.01 {
        MiniSectorState::Gain
    } else if delta_seconds >= 0.01 {
        MiniSectorState::Loss
    } else {
        MiniSectorState::Neutral
    }
}

fn interpolate_points(points: &[ReferencePoint], progress: f64) -> ReferencePoint {
    let first = points[0];
    let last = points[points.len() - 1];

    if progress <= first.progress {
        return ReferencePoint { progress, ..first };
    }
    if progress >= last.progress {
        return ReferencePoint { progress, ..last };
    }

    let upper = points.partition_point(|point| point.progress < progress);
    let before = points[upper.saturating_sub(1)];
    let after = points[upper];
    let span = after.progress - before.progress;
    if span <= f64::EPSILON {
        return ReferencePoint { progress, ..before };
    }

    let amount = (progress - before.progress) / span;
    ReferencePoint {
        progress,
        time_seconds: lerp(before.time_seconds, after.time_seconds, amount),
        throttle: lerp(before.throttle, after.throttle, amount),
        brake: lerp(before.brake, after.brake, amount),
        speed_kph: lerp(before.speed_kph, after.speed_kph, amount),
        gear: if amount < 0.5 {
            before.gear
        } else {
            after.gear
        },
        steering: lerp(before.steering, after.steering, amount),
    }
}

fn is_lap_sample_valid(snapshot: TelemetrySnapshot) -> bool {
    snapshot.game_phase == lmu_telemetry::GamePhase::GreenFlag
        && !snapshot.in_pits
        && !snapshot.in_garage
        && snapshot.lap_invalidated != Some(true)
        && snapshot
            .lap_time_seconds
            .is_some_and(|time| time.is_finite() && time >= 0.0)
        && snapshot
            .lap_progress
            .is_some_and(|progress| progress.is_finite())
}

fn is_better(current: &Option<ReferenceLap>, candidate: &ReferenceLap) -> bool {
    current
        .as_ref()
        .is_none_or(|current| candidate.total_time_seconds < current.total_time_seconds)
}

fn gear_number(gear: lmu_telemetry::Gear) -> i32 {
    match gear {
        lmu_telemetry::Gear::Reverse => -1,
        lmu_telemetry::Gear::Neutral => 0,
        lmu_telemetry::Gear::Forward(value) => i32::from(value),
        lmu_telemetry::Gear::Unknown(value) => value,
    }
}

fn lerp(start: f64, end: f64, amount: f64) -> f64 {
    start + (end - start) * amount
}

#[cfg(test)]
mod tests {
    use super::*;
    use lmu_telemetry::Gear;

    #[test]
    fn interpolates_reference_lap_by_progress() {
        let lap = reference_lap(100.0);
        let point = lap.sample_at(0.25).unwrap();

        assert_eq!(point.time_seconds, 25.0);
        assert_eq!(point.speed_kph, 125.0);
        assert_eq!(lap.points.len(), NORMALIZED_REFERENCE_POINTS);
    }

    #[test]
    fn computes_delta_and_predicted_lap() {
        let mut engine = LapEngine::default().with_personal_best(Some(reference_lap(100.0)));
        let analysis = engine.update(snapshot(1, 0.5, 52.0, 0.0, 0.0));

        assert_eq!(analysis.delta_seconds, Some(2.0));
        assert_eq!(analysis.predicted_lap_seconds, Some(102.0));
        assert_eq!(analysis.reference_lap_seconds, Some(100.0));
    }

    #[test]
    fn replaces_session_and_personal_best_when_lap_changes() {
        let mut engine = LapEngine::new(LapEngineConfig {
            min_reference_points: 2,
            ..LapEngineConfig::default()
        });

        engine.update(snapshot(1, 0.1, 10.0, 0.0, 0.0));
        engine.update(snapshot(1, 0.9, 90.0, 0.0, 0.0));
        engine.update(snapshot(2, 0.1, 8.0, 0.0, 0.0));

        assert_eq!(
            engine.session_best().map(|lap| lap.total_time_seconds),
            Some(90.0)
        );
        assert_eq!(
            engine.personal_best().map(|lap| lap.total_time_seconds),
            Some(90.0)
        );
        assert_eq!(
            engine
                .take_new_personal_best()
                .map(|lap| lap.total_time_seconds),
            Some(90.0)
        );
        assert!(engine.take_new_personal_best().is_none());
    }

    #[test]
    fn reports_brake_point_difference_in_meters() {
        let reference = ReferenceLap::new(
            100.0,
            vec![point(0.40, 40.0, 0.0, 0.0), point(0.50, 50.0, 0.0, 0.2)],
        )
        .unwrap();
        let mut engine = LapEngine::new(LapEngineConfig {
            min_reference_points: 2,
            event_match_tolerance_m: 500.0,
            ..LapEngineConfig::default()
        })
        .with_personal_best(Some(reference));

        engine.update(snapshot(1, 0.35, 35.0, 0.0, 0.0));
        let analysis = engine.update(snapshot(1, 0.40, 40.0, 0.2, 0.0));

        assert!((analysis.brake_hint_meters.unwrap() - 250.0).abs() < 0.001);
    }

    #[test]
    fn maps_progress_to_mini_sector() {
        let mut engine = LapEngine::new(LapEngineConfig {
            mini_sectors: 40,
            ..LapEngineConfig::default()
        });

        let analysis = engine.update(snapshot(1, 0.5, 50.0, 0.0, 0.0));

        assert_eq!(analysis.mini_sector_index, Some(20));
    }

    #[test]
    fn reports_last_completed_mini_sector_delta() {
        let mut engine = LapEngine::new(LapEngineConfig {
            mini_sectors: 40,
            ..LapEngineConfig::default()
        })
        .with_personal_best(Some(reference_lap(100.0)));

        let first = engine.update(snapshot(1, 0.10, 11.0, 0.0, 0.0));
        let second = engine.update(snapshot(1, 0.25, 27.0, 0.0, 0.0));

        assert_eq!(first.mini_sector_delta_seconds, None);
        assert_eq!(second.mini_sector_delta_seconds, Some(1.0));
        assert_eq!(second.mini_sector_history.len(), 1);
        assert_eq!(second.mini_sector_history[0].delta_seconds, 1.0);
    }

    #[test]
    fn does_not_save_invalid_laps_as_best_references() {
        let mut engine = LapEngine::new(LapEngineConfig {
            min_reference_points: 2,
            ..LapEngineConfig::default()
        });

        engine.update(snapshot(1, 0.1, 10.0, 0.0, 0.0));
        let mut invalid = snapshot(1, 0.9, 90.0, 0.0, 0.0);
        invalid.in_pits = true;
        engine.update(invalid);
        engine.update(snapshot(2, 0.1, 8.0, 0.0, 0.0));

        assert!(engine.session_best().is_none());
        assert!(engine.personal_best().is_none());
        assert!(engine.take_new_personal_best().is_none());
    }

    #[test]
    fn resets_session_state_when_lap_number_regresses() {
        let mut engine = LapEngine::new(LapEngineConfig {
            min_reference_points: 2,
            ..LapEngineConfig::default()
        });

        engine.update(snapshot(3, 0.1, 10.0, 0.0, 0.0));
        engine.update(snapshot(3, 0.9, 90.0, 0.0, 0.0));
        engine.update(snapshot(4, 0.1, 8.0, 0.0, 0.0));
        assert!(engine.session_best().is_some());

        engine.update(snapshot(1, 0.1, 4.0, 0.0, 0.0));

        assert!(engine.session_best().is_none());
    }

    #[test]
    fn invalidated_lap_does_not_update_valid_references() {
        let mut engine = LapEngine::new(LapEngineConfig {
            min_reference_points: 2,
            ..LapEngineConfig::default()
        });

        engine.update(snapshot(1, 0.1, 10.0, 0.0, 0.0));
        let mut invalid = snapshot(1, 0.9, 90.0, 0.0, 0.0);
        invalid.lap_invalidated = Some(true);
        engine.update(invalid);
        engine.update(snapshot(2, 0.1, 8.0, 0.0, 0.0));

        assert!(engine.session_best().is_none());
        assert!(engine.personal_best().is_none());
    }

    #[test]
    fn best_valid_lap_keeps_fastest_valid_lap_in_session() {
        let mut engine = LapEngine::new(LapEngineConfig {
            reference_mode: ReferenceMode::BestValidLap,
            min_reference_points: 2,
            ..LapEngineConfig::default()
        });

        engine.update(snapshot(1, 0.1, 10.0, 0.0, 0.0));
        engine.update(snapshot(1, 0.9, 90.0, 0.0, 0.0));
        engine.update(snapshot(2, 0.1, 9.0, 0.0, 0.0));
        engine.update(snapshot(2, 0.9, 95.0, 0.0, 0.0));
        let analysis = engine.update(snapshot(3, 0.5, 50.0, 0.0, 0.0));

        assert_eq!(analysis.reference_lap_seconds, Some(90.0));
        assert_eq!(analysis.delta_seconds, Some(5.0));
    }

    #[test]
    fn matches_brake_hint_to_nearby_corner_event() {
        let reference = ReferenceLap::new(
            100.0,
            vec![
                point(0.18, 18.0, 0.0, 0.0),
                point(0.20, 20.0, 0.0, 0.2),
                point(0.22, 22.0, 0.0, 0.0),
                point(0.43, 43.0, 0.0, 0.0),
                point(0.45, 45.0, 0.0, 0.2),
                point(0.47, 47.0, 0.0, 0.0),
                point(0.71, 71.0, 0.0, 0.0),
                point(0.73, 73.0, 0.0, 0.2),
                point(0.75, 75.0, 0.0, 0.0),
            ],
        )
        .unwrap();
        let mut engine = LapEngine::new(LapEngineConfig {
            min_reference_points: 2,
            ..LapEngineConfig::default()
        })
        .with_personal_best(Some(reference));

        engine.update(snapshot(1, 0.46, 46.0, 0.0, 0.0));
        let analysis = engine.update(snapshot(1, 0.48, 48.0, 0.2, 0.0));

        assert!((analysis.brake_hint_meters.unwrap() + 200.0).abs() < 25.0);
    }

    #[test]
    fn ignores_reference_events_outside_corner_tolerance() {
        let reference = ReferenceLap::new(
            100.0,
            vec![point(0.18, 18.0, 0.0, 0.0), point(0.20, 20.0, 0.0, 0.2)],
        )
        .unwrap();
        let mut engine = LapEngine::new(LapEngineConfig {
            min_reference_points: 2,
            event_match_tolerance_m: 50.0,
            ..LapEngineConfig::default()
        })
        .with_personal_best(Some(reference));

        engine.update(snapshot(1, 0.44, 44.0, 0.0, 0.0));
        let analysis = engine.update(snapshot(1, 0.46, 46.0, 0.2, 0.0));

        assert_eq!(analysis.brake_hint_meters, None);
    }

    fn reference_lap(total_time_seconds: f64) -> ReferenceLap {
        ReferenceLap::new(
            total_time_seconds,
            vec![
                point(0.0, 0.0, 0.0, 0.0),
                point(0.5, 50.0, 0.5, 0.0),
                point(1.0, total_time_seconds, 1.0, 0.0),
            ],
        )
        .unwrap()
    }

    fn snapshot(
        lap_number: i32,
        progress: f64,
        lap_time_seconds: f64,
        brake: f64,
        throttle: f64,
    ) -> TelemetrySnapshot {
        TelemetrySnapshot {
            throttle,
            brake,
            clutch: 0.0,
            steering: 0.0,
            rpm: 7_000.0,
            gear: Gear::Forward(4),
            speed_kph: 180.0,
            lap_distance_m: Some(progress * 5_000.0),
            track_length_m: Some(5_000.0),
            lap_time_seconds: Some(lap_time_seconds),
            lap_progress: Some(progress),
            lap_number,
            sector: 1,
            current_sector1_seconds: None,
            current_sector2_seconds: None,
            last_sector1_seconds: None,
            last_sector2_seconds: None,
            last_sector3_seconds: None,
            best_sector1_seconds: None,
            best_sector2_seconds: None,
            best_sector3_seconds: None,
            vehicle: lmu_telemetry::VehicleSystems::default(),
            wheels: lmu_telemetry::Wheels::default(),
            session: lmu_telemetry::SessionData::default(),
            session_elapsed_seconds: f64::from(lap_number) * 1_000.0 + lap_time_seconds,
            session_kind: lmu_telemetry::SessionKind::Practice,
            game_phase: lmu_telemetry::GamePhase::GreenFlag,
            in_pits: false,
            in_garage: false,
            lap_invalidated: None,
            player_slot_id: 42,
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

    fn point(progress: f64, time_seconds: f64, throttle: f64, brake: f64) -> ReferencePoint {
        ReferencePoint {
            progress,
            time_seconds,
            throttle,
            brake,
            speed_kph: 100.0 + time_seconds,
            gear: 4,
            steering: 0.0,
        }
    }
}

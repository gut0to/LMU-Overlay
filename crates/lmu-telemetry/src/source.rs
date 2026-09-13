use std::{
    error::Error,
    fmt,
    mem::size_of,
    sync::Arc,
    time::{Duration, Instant},
};

use crate::raw;
use crate::{
    GamePhase, Gear, SessionData, SessionKind, TelemetryMetadata, TelemetrySample, VehicleSystems,
    WheelData, Wheels,
};

#[cfg(windows)]
use std::slice;

pub trait TelemetrySource {
    fn is_available(&self) -> bool;
    fn read_sample(&mut self) -> Result<Option<TelemetrySample>, TelemetryError>;
}

#[derive(Debug)]
pub enum TelemetryError {
    UnsupportedPlatform,
    MappingFailed,
    BufferTooSmall,
    TornFrame,
}

impl fmt::Display for TelemetryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TelemetryError::UnsupportedPlatform => {
                f.write_str("shared memory reading is only supported on Windows")
            }
            TelemetryError::MappingFailed => {
                f.write_str("could not map the telemetry shared memory buffer")
            }
            TelemetryError::BufferTooSmall => {
                f.write_str("telemetry shared memory buffer is smaller than expected")
            }
            TelemetryError::TornFrame => {
                f.write_str("telemetry frame changed while it was being read")
            }
        }
    }
}

impl Error for TelemetryError {}

const TELEMETRY_MAP_NAME: &str = "LMU_Data";
const MAX_VEHICLES: usize = 104;
const BUFFER_SIZE: usize = 324_820;

const OFFSET_SCORING_DATA: usize = 1_632;
const OFFSET_TELEMETRY_DATA: usize = 128_464;

const SCORING_INFO_SIZE: usize = 548;
const VEHICLE_SCORING_SIZE: usize = 584;
const VEHICLE_TELEMETRY_SIZE: usize = 1_888;

const OFFSET_GAME_VERSION: usize = 64;
const OFFSET_SCORING_CURRENT_ET: usize = OFFSET_SCORING_DATA + 68;
const OFFSET_TRACK_LENGTH: usize = OFFSET_SCORING_DATA + 88;
const OFFSET_SCORING_NUM_VEHICLES: usize = OFFSET_SCORING_DATA + 104;
const OFFSET_SCORING_GAME_PHASE: usize = OFFSET_SCORING_DATA + 108;
const OFFSET_TELEMETRY_ACTIVE_VEHICLES: usize = OFFSET_TELEMETRY_DATA;
const OFFSET_TELEMETRY_PLAYER_INDEX: usize = OFFSET_TELEMETRY_DATA + 1;
const OFFSET_TELEMETRY_PLAYER_HAS_VEHICLE: usize = OFFSET_TELEMETRY_DATA + 2;
const OFFSET_SCORING_VEHICLES: usize = OFFSET_SCORING_DATA + SCORING_INFO_SIZE + 12;
const OFFSET_TELEMETRY_VEHICLES: usize = OFFSET_TELEMETRY_DATA + 4;

const OFFSET_SCORING_TRACK_NAME: usize = OFFSET_SCORING_DATA;
const OFFSET_SCORING_SESSION: usize = OFFSET_SCORING_DATA + 64;
const OFFSET_ELAPSED_TIME: usize = 12;
const OFFSET_TELEMETRY_SLOT_ID: usize = 0;
const OFFSET_LAP_NUMBER: usize = 20;
const OFFSET_LAP_START_ET: usize = 24;
const OFFSET_TELEMETRY_VEHICLE_NAME: usize = 32;
const OFFSET_TELEMETRY_TRACK_NAME: usize = 96;
const OFFSET_GEAR: usize = 352;
const OFFSET_RPM: usize = 356;
const OFFSET_THROTTLE: usize = 388;
const OFFSET_BRAKE: usize = 396;
const OFFSET_STEERING: usize = 404;
const OFFSET_CLUTCH: usize = 412;
const OFFSET_SECTOR: usize = 600;

const OFFSET_SCORING_SLOT_ID: usize = 0;
const OFFSET_SCORING_VEHICLE_NAME: usize = 36;
const OFFSET_SCORING_SECTOR: usize = 102;
const OFFSET_SCORING_LAP_DISTANCE: usize = 104;
const OFFSET_SCORING_IS_PLAYER: usize = 196;
const OFFSET_SCORING_CONTROL: usize = 197;
const OFFSET_SCORING_IN_PITS: usize = 198;
const OFFSET_SCORING_VEHICLE_CLASS: usize = 200;
const OFFSET_SCORING_IN_GARAGE_STALL: usize = 507;

const MAX_TORN_FRAME_RETRIES: usize = 3;

const OFFSET_SCORING_END_ET: usize = OFFSET_SCORING_DATA + raw::session::END_ET;
const OFFSET_SCORING_MAX_LAPS: usize = OFFSET_SCORING_DATA + raw::session::MAX_LAPS;
const OFFSET_SCORING_YELLOW_FLAG: usize = OFFSET_SCORING_DATA + raw::session::YELLOW_FLAG;
const OFFSET_SCORING_SECTOR_FLAGS: usize = OFFSET_SCORING_DATA + raw::session::SECTOR_FLAGS;
const OFFSET_SCORING_START_LIGHT: usize = OFFSET_SCORING_DATA + raw::session::START_LIGHT;
const OFFSET_SCORING_AMBIENT_TEMP: usize = OFFSET_SCORING_DATA + raw::session::AMBIENT_TEMP;
const OFFSET_SCORING_TRACK_TEMP: usize = OFFSET_SCORING_DATA + raw::session::TRACK_TEMP;
const OFFSET_SCORING_RAINING: usize = OFFSET_SCORING_DATA + raw::session::RAINING;
const OFFSET_SCORING_MAX_WETNESS: usize = OFFSET_SCORING_DATA + raw::session::MAX_WETNESS;
const OFFSET_SCORING_SESSION_REMAINING: usize =
    OFFSET_SCORING_DATA + raw::session::SESSION_REMAINING;
const OFFSET_SCORING_TIME_OF_DAY: usize = OFFSET_SCORING_DATA + raw::session::TIME_OF_DAY;
const OFFSET_SCORING_CLOUD_COVERAGE: usize = OFFSET_SCORING_DATA + raw::session::CLOUD_COVERAGE;
const OFFSET_SCORING_TRACK_GRIP: usize = OFFSET_SCORING_DATA + raw::session::TRACK_GRIP;

const OFFSET_ENGINE_WATER_TEMP: usize = raw::telemetry::WATER_TEMP;
const OFFSET_ENGINE_OIL_TEMP: usize = raw::telemetry::OIL_TEMP;
const OFFSET_STEERING_TORQUE: usize = raw::telemetry::STEERING_TORQUE;
const OFFSET_FUEL: usize = raw::telemetry::FUEL;
const OFFSET_MAX_RPM: usize = raw::telemetry::MAX_RPM;
const OFFSET_SCHEDULED_STOPS: usize = raw::telemetry::SCHEDULED_STOPS;
const OFFSET_OVERHEATING: usize = raw::telemetry::OVERHEATING;
const OFFSET_HEADLIGHTS: usize = raw::telemetry::HEADLIGHTS;
const OFFSET_ENGINE_TORQUE: usize = raw::telemetry::ENGINE_TORQUE;
const OFFSET_FUEL_CAPACITY: usize = raw::telemetry::FUEL_CAPACITY;
const OFFSET_REAR_BRAKE_BIAS: usize = raw::telemetry::REAR_BRAKE_BIAS;
const OFFSET_TURBO_BOOST: usize = raw::telemetry::TURBO_BOOST;
const OFFSET_BATTERY: usize = raw::telemetry::BATTERY_CHARGE_FRACTION;
const OFFSET_ELECTRIC_TORQUE: usize = raw::telemetry::ELECTRIC_MOTOR_TORQUE;
const OFFSET_ELECTRIC_RPM: usize = raw::telemetry::ELECTRIC_MOTOR_RPM;
const OFFSET_ELECTRIC_TEMP: usize = raw::telemetry::ELECTRIC_MOTOR_TEMP;
const OFFSET_ELECTRIC_WATER_TEMP: usize = raw::telemetry::ELECTRIC_WATER_TEMP;
const OFFSET_ELECTRIC_STATE: usize = raw::telemetry::ELECTRIC_MOTOR_STATE;
const OFFSET_LAP_INVALIDATED: usize = raw::telemetry::LAP_INVALIDATED;
const OFFSET_ABS_ACTIVE: usize = raw::telemetry::ABS_ACTIVE;
const OFFSET_TC_ACTIVE: usize = raw::telemetry::TC_ACTIVE;
const OFFSET_SPEED_LIMITER_ACTIVE: usize = raw::telemetry::SPEED_LIMITER_ACTIVE;
const OFFSET_TC: usize = raw::telemetry::TC;
const OFFSET_TC_SLIP: usize = raw::telemetry::TC_SLIP;
const OFFSET_TC_CUT: usize = raw::telemetry::TC_CUT;
const OFFSET_ABS: usize = raw::telemetry::ABS;
const OFFSET_ABS_MAX: usize = raw::telemetry::ABS_MAX;
const OFFSET_MOTOR_MAP: usize = raw::telemetry::MOTOR_MAP;
const OFFSET_MOTOR_MAP_MAX: usize = raw::telemetry::MOTOR_MAP_MAX;
const OFFSET_MIGRATION: usize = raw::telemetry::MIGRATION;
const OFFSET_MIGRATION_MAX: usize = raw::telemetry::MIGRATION_MAX;
const OFFSET_REGEN: usize = raw::telemetry::REGEN;
const OFFSET_VIRTUAL_ENERGY: usize = raw::telemetry::VIRTUAL_ENERGY;
const OFFSET_GAP_CAR_AHEAD: usize = raw::telemetry::GAP_CAR_AHEAD;
const OFFSET_GAP_CAR_BEHIND: usize = raw::telemetry::GAP_CAR_BEHIND;
const OFFSET_WHEELS: usize = raw::telemetry::WHEELS;

pub struct SharedMemoryTelemetrySource {
    inner: PlatformTelemetrySource,
    next_reconnect_attempt: Instant,
}

impl SharedMemoryTelemetrySource {
    pub fn open() -> Result<Self, TelemetryError> {
        Ok(Self {
            inner: PlatformTelemetrySource::open()?,
            next_reconnect_attempt: Instant::now(),
        })
    }
}

impl TelemetrySource for SharedMemoryTelemetrySource {
    fn is_available(&self) -> bool {
        self.inner.is_available()
    }

    fn read_sample(&mut self) -> Result<Option<TelemetrySample>, TelemetryError> {
        if !self.inner.is_available() {
            if Instant::now() < self.next_reconnect_attempt {
                return Ok(None);
            }
            self.inner = PlatformTelemetrySource::open()?;
            self.next_reconnect_attempt = Instant::now() + Duration::from_millis(500);
        }

        self.inner.read_sample()
    }
}

#[cfg(windows)]
struct PlatformTelemetrySource {
    handle: windows_sys::Win32::Foundation::HANDLE,
    view: windows_sys::Win32::System::Memory::MEMORY_MAPPED_VIEW_ADDRESS,
    cached_field: Arc<[crate::VehicleScoringSnapshot]>,
    last_scoring_read: Instant,
}

// The mapping is opened read-only and is owned by this wrapper. Moving the
// wrapper to the acquisition thread does not create shared mutable access.
#[cfg(windows)]
unsafe impl Send for PlatformTelemetrySource {}

#[cfg(windows)]
impl PlatformTelemetrySource {
    fn open() -> Result<Self, TelemetryError> {
        use std::os::windows::ffi::OsStrExt;
        use std::{ffi::OsStr, ptr};
        use windows_sys::Win32::System::Memory::{
            MapViewOfFile, OpenFileMappingW, FILE_MAP_READ, MEMORY_MAPPED_VIEW_ADDRESS,
        };

        let wide_name: Vec<u16> = OsStr::new(TELEMETRY_MAP_NAME)
            .encode_wide()
            .chain([0])
            .collect();
        let handle = unsafe { OpenFileMappingW(FILE_MAP_READ, 0, wide_name.as_ptr()) };

        if handle.is_null() {
            return Ok(Self {
                handle: ptr::null_mut(),
                view: MEMORY_MAPPED_VIEW_ADDRESS {
                    Value: ptr::null_mut(),
                },
                cached_field: Arc::from(Vec::new()),
                last_scoring_read: Instant::now() - Duration::from_secs(1),
            });
        }

        let view = unsafe { MapViewOfFile(handle, FILE_MAP_READ, 0, 0, BUFFER_SIZE) };
        if view.Value.is_null() {
            unsafe {
                windows_sys::Win32::Foundation::CloseHandle(handle);
            }
            return Err(TelemetryError::MappingFailed);
        }

        Ok(Self {
            handle,
            view,
            cached_field: Arc::from(Vec::new()),
            last_scoring_read: Instant::now() - Duration::from_secs(1),
        })
    }

    fn is_available(&self) -> bool {
        !self.handle.is_null() && !self.view.Value.is_null()
    }

    fn read_sample(&mut self) -> Result<Option<TelemetrySample>, TelemetryError> {
        if !self.is_available() {
            return Ok(None);
        }

        let bytes = unsafe { slice::from_raw_parts(self.view.Value.cast::<u8>(), BUFFER_SIZE) };
        let refresh_scoring = self.last_scoring_read.elapsed() >= Duration::from_millis(100);
        let cached_field = (!refresh_scoring).then_some(&self.cached_field);
        let sample =
            read_consistent_sample_from_bytes(bytes, cached_field)?.map(TelemetrySample::sanitized);
        if refresh_scoring {
            if let Some(sample) = &sample {
                self.cached_field = sample.field.clone();
                self.last_scoring_read = Instant::now();
            }
        }
        Ok(sample)
    }
}

#[cfg(windows)]
impl Drop for PlatformTelemetrySource {
    fn drop(&mut self) {
        if !self.view.Value.is_null() {
            unsafe {
                windows_sys::Win32::System::Memory::UnmapViewOfFile(self.view);
            }
        }
        if !self.handle.is_null() {
            unsafe {
                windows_sys::Win32::Foundation::CloseHandle(self.handle);
            }
        }
    }
}

#[cfg(not(windows))]
struct PlatformTelemetrySource;

#[cfg(not(windows))]
impl PlatformTelemetrySource {
    fn open() -> Result<Self, TelemetryError> {
        Ok(Self)
    }

    fn is_available(&self) -> bool {
        false
    }

    fn read_sample(&mut self) -> Result<Option<TelemetrySample>, TelemetryError> {
        Err(TelemetryError::UnsupportedPlatform)
    }
}

fn read_consistent_sample_from_bytes(
    bytes: &[u8],
    cached_field: Option<&Arc<[crate::VehicleScoringSnapshot]>>,
) -> Result<Option<TelemetrySample>, TelemetryError> {
    let mut last_error = None;
    for _ in 0..MAX_TORN_FRAME_RETRIES {
        match read_sample_once(bytes, cached_field) {
            Ok(sample) => return Ok(sample),
            Err(TelemetryError::TornFrame) => last_error = Some(TelemetryError::TornFrame),
            Err(error) => return Err(error),
        }
    }

    Err(last_error.unwrap_or(TelemetryError::TornFrame))
}

#[cfg(test)]
fn read_sample_from_bytes(bytes: &[u8]) -> Result<Option<TelemetrySample>, TelemetryError> {
    read_consistent_sample_from_bytes(bytes, None)
}

fn read_sample_once(
    bytes: &[u8],
    cached_field: Option<&Arc<[crate::VehicleScoringSnapshot]>>,
) -> Result<Option<TelemetrySample>, TelemetryError> {
    if bytes.len() < BUFFER_SIZE {
        return Err(TelemetryError::BufferTooSmall);
    }

    let marker_before = read_frame_marker(bytes)?;
    if detect_layout(bytes, marker_before).is_none() {
        return Ok(None);
    }

    let active_vehicles = marker_before.active_vehicles as usize;
    let scoring_vehicles =
        read_i32(bytes, OFFSET_SCORING_NUM_VEHICLES)?.clamp(0, MAX_VEHICLES as i32) as usize;
    let player_index = marker_before.player_index as usize;
    let player_has_vehicle = marker_before.player_has_vehicle;

    if !player_has_vehicle || player_index >= active_vehicles || player_index >= MAX_VEHICLES {
        return Ok(None);
    }

    let vehicle_offset = OFFSET_TELEMETRY_VEHICLES + player_index * VEHICLE_TELEMETRY_SIZE;
    let player_slot_id = read_i32(bytes, vehicle_offset + OFFSET_TELEMETRY_SLOT_ID)?;
    let scoring_offset = find_player_scoring_offset(bytes, scoring_vehicles, player_slot_id)?;
    let field = match cached_field {
        Some(field) => Arc::clone(field),
        None => Arc::from(read_field(bytes, scoring_vehicles)?),
    };
    let mut session = read_session(bytes, scoring_offset)?;
    let (gap_ahead_seconds, gap_behind_seconds) = field_gaps(&field, player_slot_id);
    if gap_ahead_seconds.is_some() {
        session.gap_ahead_seconds = gap_ahead_seconds;
    }
    session.gap_behind_seconds = gap_behind_seconds;

    let sample = TelemetrySample {
        timestamp_seconds: read_f64(bytes, OFFSET_SCORING_CURRENT_ET)
            .or_else(|_| read_f64(bytes, vehicle_offset + OFFSET_ELAPSED_TIME))?,
        speed_mps: read_local_speed_mps(bytes, vehicle_offset)?,
        rpm: read_f64(bytes, vehicle_offset + OFFSET_RPM)?,
        gear: Gear::from(read_i32(bytes, vehicle_offset + OFFSET_GEAR)?),
        throttle: read_f64(bytes, vehicle_offset + OFFSET_THROTTLE)?,
        brake: read_f64(bytes, vehicle_offset + OFFSET_BRAKE)?,
        clutch: read_f64(bytes, vehicle_offset + OFFSET_CLUTCH)?,
        steering: read_f64(bytes, vehicle_offset + OFFSET_STEERING)?,
        lap_distance_m: scoring_offset
            .map(|offset| read_f64(bytes, offset + OFFSET_SCORING_LAP_DISTANCE))
            .transpose()?,
        track_length_m: Some(read_f64(bytes, OFFSET_TRACK_LENGTH)?),
        lap_number: read_i32(bytes, vehicle_offset + OFFSET_LAP_NUMBER)?,
        lap_start_seconds: read_f64(bytes, vehicle_offset + OFFSET_LAP_START_ET)?,
        sector: read_sector(bytes, scoring_offset, vehicle_offset)?,
        sector_times: read_sector_times(bytes, scoring_offset)?,
        vehicle: read_vehicle_systems(bytes, vehicle_offset, scoring_offset)?,
        wheels: read_wheels(bytes, vehicle_offset)?,
        session,
        metadata: read_metadata(bytes, scoring_offset, vehicle_offset, player_slot_id)?,
        field,
    };

    ensure_same_frame(marker_before, read_frame_marker(bytes)?)?;

    Ok(Some(sample.sanitized()))
}

fn ensure_same_frame(before: FrameMarker, after: FrameMarker) -> Result<(), TelemetryError> {
    if before != after {
        return Err(TelemetryError::TornFrame);
    }

    Ok(())
}

fn find_player_scoring_offset(
    bytes: &[u8],
    scoring_vehicles: usize,
    player_slot_id: i32,
) -> Result<Option<usize>, TelemetryError> {
    let mut player_flag_match = None;

    for index in 0..scoring_vehicles.min(MAX_VEHICLES) {
        let offset = OFFSET_SCORING_VEHICLES + index * VEHICLE_SCORING_SIZE;
        let slot_id = read_i32(bytes, offset + OFFSET_SCORING_SLOT_ID)?;
        if slot_id == player_slot_id {
            return Ok(Some(offset));
        }
        if read_bool(bytes, offset + OFFSET_SCORING_IS_PLAYER)?
            || read_i8(bytes, offset + OFFSET_SCORING_CONTROL)? == 0
        {
            player_flag_match = Some(offset);
        }
    }

    Ok(player_flag_match)
}

fn read_metadata(
    bytes: &[u8],
    scoring_offset: Option<usize>,
    vehicle_offset: usize,
    player_slot_id: i32,
) -> Result<TelemetryMetadata, TelemetryError> {
    let telemetry_track = read_string(bytes, vehicle_offset + OFFSET_TELEMETRY_TRACK_NAME, 64)?;
    let scoring_track = read_string(bytes, OFFSET_SCORING_TRACK_NAME, 64)?;
    let track_name = scoring_track.or(telemetry_track);
    let vehicle_name = scoring_offset
        .map(|offset| read_string(bytes, offset + OFFSET_SCORING_VEHICLE_NAME, 64))
        .transpose()?
        .flatten()
        .or(read_string(
            bytes,
            vehicle_offset + OFFSET_TELEMETRY_VEHICLE_NAME,
            64,
        )?);
    let vehicle_class = scoring_offset
        .map(|offset| read_string(bytes, offset + OFFSET_SCORING_VEHICLE_CLASS, 32))
        .transpose()?
        .flatten();

    Ok(TelemetryMetadata {
        track_name,
        track_layout: None,
        vehicle_name,
        vehicle_class,
        session_kind: SessionKind::from(read_i32(bytes, OFFSET_SCORING_SESSION)?),
        game_phase: GamePhase::from(read_u8(bytes, OFFSET_SCORING_GAME_PHASE)?),
        in_pits: scoring_offset
            .map(|offset| read_bool(bytes, offset + OFFSET_SCORING_IN_PITS))
            .transpose()?
            .unwrap_or(false),
        in_garage: scoring_offset
            .map(|offset| read_bool(bytes, offset + OFFSET_SCORING_IN_GARAGE_STALL))
            .transpose()?
            .unwrap_or(false),
        lap_invalidated: read_bool(bytes, vehicle_offset + OFFSET_LAP_INVALIDATED).ok(),
        player_slot_id: scoring_offset
            .map(|offset| read_i32(bytes, offset + OFFSET_SCORING_SLOT_ID))
            .transpose()?
            .unwrap_or(player_slot_id),
    })
}

fn read_sector(
    bytes: &[u8],
    scoring_offset: Option<usize>,
    vehicle_offset: usize,
) -> Result<i32, TelemetryError> {
    if let Some(offset) = scoring_offset {
        return Ok(i32::from(read_i8(bytes, offset + OFFSET_SCORING_SECTOR)?));
    }

    read_i32(bytes, vehicle_offset + OFFSET_SECTOR)
}

fn read_local_speed_mps(bytes: &[u8], vehicle_offset: usize) -> Result<f64, TelemetryError> {
    let velocity = raw::telemetry::LOCAL_VELOCITY;
    let x = read_f64(bytes, vehicle_offset + velocity)?;
    let y = read_f64(bytes, vehicle_offset + velocity + 8)?;
    let z = read_f64(bytes, vehicle_offset + velocity + 16)?;
    Ok((x * x + y * y + z * z).sqrt())
}

fn read_sector_times(
    bytes: &[u8],
    scoring_offset: Option<usize>,
) -> Result<crate::SectorTimes, TelemetryError> {
    let Some(offset) = scoring_offset else {
        return Ok(crate::SectorTimes::default());
    };
    let current_s1 = valid_time(read_f64(bytes, offset + raw::scoring::CURRENT_SECTOR1)?);
    let current_s2_cumulative =
        valid_time(read_f64(bytes, offset + raw::scoring::CURRENT_SECTOR2)?);
    let last_s1 = valid_time(read_f64(bytes, offset + raw::scoring::LAST_SECTOR1)?);
    let last_s2 = valid_time(read_f64(bytes, offset + raw::scoring::LAST_SECTOR2)?);
    let last_lap = valid_time(read_f64(bytes, offset + raw::scoring::LAST_LAP)?);
    let best_s1 = valid_time(read_f64(bytes, offset + raw::scoring::BEST_SECTOR1)?);
    let best_s2 = valid_time(read_f64(bytes, offset + raw::scoring::BEST_SECTOR2)?);
    let best_lap = valid_time(read_f64(bytes, offset + raw::scoring::BEST_LAP)?);
    Ok(crate::SectorTimes {
        current_sector1_seconds: current_s1,
        current_sector2_seconds: subtract_sector(current_s2_cumulative, current_s1),
        last_sector1_seconds: last_s1,
        last_sector2_seconds: subtract_sector(last_s2, last_s1),
        last_sector3_seconds: subtract_sector(last_lap, last_s2),
        best_sector1_seconds: best_s1,
        best_sector2_seconds: subtract_sector(best_s2, best_s1),
        best_sector3_seconds: subtract_sector(best_lap, best_s2),
    })
}

fn read_vehicle_systems(
    bytes: &[u8],
    offset: usize,
    scoring_offset: Option<usize>,
) -> Result<VehicleSystems, TelemetryError> {
    let hybrid_state = read_u8(bytes, offset + OFFSET_ELECTRIC_STATE)?;
    let hybrid = (hybrid_state != 0).then_some(());
    let mut dent_severity = [None; 8];
    for (index, value) in dent_severity.iter_mut().enumerate() {
        *value = Some(read_u8(
            bytes,
            offset + raw::telemetry::DENT_SEVERITY + index,
        )?);
    }
    Ok(VehicleSystems {
        max_rpm: positive(read_f64(bytes, offset + OFFSET_MAX_RPM)?),
        fuel_liters: positive(read_f64(bytes, offset + OFFSET_FUEL)?),
        fuel_capacity_liters: positive(read_f64(bytes, offset + OFFSET_FUEL_CAPACITY)?),
        engine_water_temp_c: finite(read_f64(bytes, offset + OFFSET_ENGINE_WATER_TEMP)?),
        engine_oil_temp_c: finite(read_f64(bytes, offset + OFFSET_ENGINE_OIL_TEMP)?),
        engine_torque_nm: finite(read_f64(bytes, offset + OFFSET_ENGINE_TORQUE)?),
        turbo_boost_kpa: finite(read_f64(bytes, offset + OFFSET_TURBO_BOOST)?),
        brake_bias_front_percent: finite(read_f64(bytes, offset + OFFSET_REAR_BRAKE_BIAS)?)
            .map(|rear| (1.0 - rear) * 100.0),
        speed_limiter_active: Some(read_bool(bytes, offset + OFFSET_SPEED_LIMITER_ACTIVE)?),
        drs_active: scoring_offset
            .map(|scoring| read_u8(bytes, scoring + raw::scoring::DRS_STATE))
            .transpose()?
            .map(|state| state != 0),
        wiper_state: Some(read_u8(bytes, offset + raw::telemetry::WIPER_STATE)?),
        lift_and_coast_progress: Some(
            read_u8(bytes, offset + raw::telemetry::LIFT_AND_COAST)? as f64
        ),
        track_limit_steps: Some(read_u8(bytes, offset + raw::telemetry::TRACK_LIMIT_STEPS)?),
        tc_active: Some(read_bool(bytes, offset + OFFSET_TC_ACTIVE)?),
        abs_active: Some(read_bool(bytes, offset + OFFSET_ABS_ACTIVE)?),
        tc_setting: Some(read_u8(bytes, offset + OFFSET_TC)? as i32),
        abs_setting: Some(read_u8(bytes, offset + OFFSET_ABS)? as i32),
        motor_map: Some(read_u8(bytes, offset + OFFSET_MOTOR_MAP)? as i32),
        battery_charge_percent: hybrid
            .map(|_| read_f64(bytes, offset + OFFSET_BATTERY))
            .transpose()?
            .map(|v| v * 100.0),
        state_of_charge_percent: hybrid
            .map(|_| read_f32(bytes, offset + raw::telemetry::STATE_OF_CHARGE))
            .transpose()?
            .and_then(|value| finite(f64::from(value))),
        virtual_energy_percent: hybrid
            .map(|_| read_f32(bytes, offset + OFFSET_VIRTUAL_ENERGY))
            .transpose()?
            .map(|v| f64::from(v) * 100.0),
        hybrid_regen_active: hybrid.map(|_| hybrid_state == 3),
        scheduled_stops: Some(read_u8(bytes, offset + OFFSET_SCHEDULED_STOPS)?),
        overheating: Some(read_bool(bytes, offset + OFFSET_OVERHEATING)?),
        headlights: Some(read_bool(bytes, offset + OFFSET_HEADLIGHTS)?),
        body_detached: Some(read_bool(bytes, offset + raw::telemetry::DETACHED)?),
        dent_severity,
        last_impact_et: finite(read_f64(bytes, offset + raw::telemetry::LAST_IMPACT_ET)?),
        last_impact_magnitude: finite(read_f64(
            bytes,
            offset + raw::telemetry::LAST_IMPACT_MAGNITUDE,
        )?),
        last_impact_position: Some([
            read_f64(bytes, offset + raw::telemetry::LAST_IMPACT_POSITION)?,
            read_f64(bytes, offset + raw::telemetry::LAST_IMPACT_POSITION + 8)?,
            read_f64(bytes, offset + raw::telemetry::LAST_IMPACT_POSITION + 16)?,
        ]),
        steering_torque_nm: finite(read_f64(bytes, offset + OFFSET_STEERING_TORQUE)?),
        electric_motor_torque_nm: hybrid
            .map(|_| read_f64(bytes, offset + OFFSET_ELECTRIC_TORQUE))
            .transpose()?,
        electric_motor_rpm: hybrid
            .map(|_| read_f64(bytes, offset + OFFSET_ELECTRIC_RPM))
            .transpose()?,
        electric_motor_temp_c: hybrid
            .map(|_| read_f64(bytes, offset + OFFSET_ELECTRIC_TEMP))
            .transpose()?,
        electric_motor_water_temp_c: hybrid
            .map(|_| read_f64(bytes, offset + OFFSET_ELECTRIC_WATER_TEMP))
            .transpose()?,
        electric_motor_state: hybrid.map(|_| hybrid_state),
        tc_slip: Some(read_u8(bytes, offset + OFFSET_TC_SLIP)?),
        tc_max: Some(read_u8(bytes, offset + raw::telemetry::TC_MAX)?),
        tc_slip_max: Some(read_u8(bytes, offset + raw::telemetry::TC_SLIP_MAX)?),
        tc_cut: Some(read_u8(bytes, offset + OFFSET_TC_CUT)?),
        tc_cut_max: Some(read_u8(bytes, offset + raw::telemetry::TC_CUT_MAX)?),
        abs_max: Some(read_u8(bytes, offset + OFFSET_ABS_MAX)?),
        motor_map_max: Some(read_u8(bytes, offset + OFFSET_MOTOR_MAP_MAX)?),
        migration: Some(read_u8(bytes, offset + OFFSET_MIGRATION)?),
        migration_max: Some(read_u8(bytes, offset + OFFSET_MIGRATION_MAX)?),
        front_anti_sway: Some(read_u8(bytes, offset + raw::telemetry::FRONT_ANTI_SWAY)?),
        front_anti_sway_max: Some(read_u8(
            bytes,
            offset + raw::telemetry::FRONT_ANTI_SWAY_MAX,
        )?),
        rear_anti_sway: Some(read_u8(bytes, offset + raw::telemetry::REAR_ANTI_SWAY)?),
        rear_anti_sway_max: Some(read_u8(bytes, offset + raw::telemetry::REAR_ANTI_SWAY_MAX)?),
        regen_kw: hybrid
            .map(|_| read_f32(bytes, offset + OFFSET_REGEN))
            .transpose()?
            .map(f64::from),
        gap_car_ahead_seconds: finite(read_f32(bytes, offset + OFFSET_GAP_CAR_AHEAD)? as f64),
        gap_car_behind_seconds: finite(read_f32(bytes, offset + OFFSET_GAP_CAR_BEHIND)? as f64),
    })
}

fn read_wheels(bytes: &[u8], vehicle_offset: usize) -> Result<Wheels, TelemetryError> {
    let mut wheels = [WheelData::default(); 4];
    for (index, wheel) in wheels.iter_mut().enumerate() {
        *wheel = read_wheel(
            bytes,
            vehicle_offset + OFFSET_WHEELS + index * raw::wheel::SIZE,
        )?;
    }
    Ok(Wheels {
        front_left: wheels[0],
        front_right: wheels[1],
        rear_left: wheels[2],
        rear_right: wheels[3],
    })
}

fn read_wheel(bytes: &[u8], offset: usize) -> Result<WheelData, TelemetryError> {
    let temp = |index: usize| {
        read_f64(bytes, offset + raw::wheel::TEMPERATURE + index * 8)
            .ok()
            .and_then(kelvin_to_celsius)
    };
    let inner = |index: usize| {
        read_f64(bytes, offset + raw::wheel::INNER_TEMP + index * 8)
            .ok()
            .and_then(kelvin_to_celsius)
    };
    Ok(WheelData {
        pressure_kpa: positive(read_f64(bytes, offset + raw::wheel::PRESSURE)?),
        surface_temp_left_c: temp(0),
        surface_temp_center_c: temp(1),
        surface_temp_right_c: temp(2),
        carcass_temp_c: kelvin_to_celsius(read_f64(bytes, offset + raw::wheel::CARCASS_TEMP)?),
        wear_percent: finite(read_f64(bytes, offset + raw::wheel::WEAR)?).map(|v| v * 100.0),
        brake_temp_c: finite(read_f64(bytes, offset + raw::wheel::BRAKE_TEMP)?),
        brake_pressure_kpa: finite(read_f64(bytes, offset + raw::wheel::BRAKE_PRESSURE)?),
        grip_fraction: finite(read_f64(bytes, offset + 112)?),
        detached: Some(read_bool(bytes, offset + raw::wheel::DETACHED)?),
        flat: Some(read_bool(bytes, offset + raw::wheel::FLAT)?),
        inner_temp_c: [inner(0), inner(1), inner(2)]
            .into_iter()
            .flatten()
            .reduce(|a, b| a + b)
            .map(|v| v / 3.0),
        optimal_temp_c: Some(read_f32(bytes, offset + raw::wheel::OPTIMAL_TEMP)? as f64),
        compound_index: Some(read_u8(bytes, offset + raw::wheel::COMPOUND_INDEX)?),
        compound_type: Some(read_u8(bytes, offset + raw::wheel::COMPOUND_TYPE)?),
        surface_type: Some(read_u8(bytes, offset + raw::wheel::SURFACE_TYPE)?),
        suspension_deflection_m: finite(read_f64(
            bytes,
            offset + raw::wheel::SUSPENSION_DEFLECTION,
        )?),
        ride_height_m: finite(read_f64(bytes, offset + raw::wheel::RIDE_HEIGHT)?),
        suspension_force_n: finite(read_f64(bytes, offset + raw::wheel::SUSPENSION_FORCE)?),
        rotation_rad_s: finite(read_f64(bytes, offset + raw::wheel::ROTATION)?),
        camber_rad: finite(read_f64(bytes, offset + raw::wheel::CAMBER)?),
        tyre_load_n: finite(read_f64(bytes, offset + raw::wheel::TYRE_LOAD)?),
    })
}

fn read_session(
    bytes: &[u8],
    scoring_offset: Option<usize>,
) -> Result<SessionData, TelemetryError> {
    let position = scoring_offset
        .map(|offset| read_u8(bytes, offset + raw::scoring::PLACE))
        .transpose()?;
    let gap_ahead = scoring_offset
        .map(|offset| read_f64(bytes, offset + raw::scoring::TIME_BEHIND_NEXT))
        .transpose()?
        .and_then(finite);
    let penalties = scoring_offset
        .map(|offset| read_i16(bytes, offset + raw::scoring::NUM_PENALTIES))
        .transpose()?
        .map(i32::from);
    Ok(SessionData {
        position: position.map(i32::from).filter(|value| *value > 0),
        total_vehicles: Some(read_i32(bytes, OFFSET_SCORING_NUM_VEHICLES)?),
        flag: scoring_offset
            .map(|offset| read_u8(bytes, offset + raw::scoring::FLAG))
            .transpose()?
            .map(i32::from),
        gap_ahead_seconds: gap_ahead,
        gap_behind_seconds: None,
        session_remaining_seconds: finite(read_f32(bytes, OFFSET_SCORING_SESSION_REMAINING)? as f64),
        ambient_temp_c: finite(read_f64(bytes, OFFSET_SCORING_AMBIENT_TEMP)?),
        track_temp_c: finite(read_f64(bytes, OFFSET_SCORING_TRACK_TEMP)?),
        rain_density: finite(read_f64(bytes, OFFSET_SCORING_RAINING)?),
        track_wetness: finite(read_f64(bytes, OFFSET_SCORING_MAX_WETNESS)?),
        max_laps: Some(read_i32(bytes, OFFSET_SCORING_MAX_LAPS)?),
        end_time_seconds: finite(read_f64(bytes, OFFSET_SCORING_END_ET)?),
        yellow_flag_state: Some(read_i8(bytes, OFFSET_SCORING_YELLOW_FLAG)?),
        sector_flags: [
            Some(read_u8(bytes, OFFSET_SCORING_SECTOR_FLAGS)?),
            Some(read_u8(bytes, OFFSET_SCORING_SECTOR_FLAGS + 1)?),
            Some(read_u8(bytes, OFFSET_SCORING_SECTOR_FLAGS + 2)?),
        ],
        start_light: Some(read_u8(bytes, OFFSET_SCORING_START_LIGHT)?),
        time_of_day: finite(read_f32(bytes, OFFSET_SCORING_TIME_OF_DAY)? as f64),
        cloud_coverage: Some(read_u8(bytes, OFFSET_SCORING_CLOUD_COVERAGE)?),
        track_grip_level: Some(read_u8(bytes, OFFSET_SCORING_TRACK_GRIP)?),
        count_lap_flag: scoring_offset
            .map(|offset| read_u8(bytes, offset + raw::scoring::COUNT_LAP_FLAG))
            .transpose()?
            .map(i32::from),
        pit_state: scoring_offset
            .map(|offset| read_u8(bytes, offset + raw::scoring::PIT_STATE))
            .transpose()?,
        penalties,
    })
}

fn read_field(
    bytes: &[u8],
    scoring_vehicles: usize,
) -> Result<Vec<crate::VehicleScoringSnapshot>, TelemetryError> {
    let mut field = Vec::with_capacity(scoring_vehicles.min(raw::MAX_VEHICLES));
    for index in 0..scoring_vehicles.min(raw::MAX_VEHICLES) {
        let offset = OFFSET_SCORING_VEHICLES + index * raw::VEHICLE_SCORING_SIZE;
        let slot_id = read_i32(bytes, offset + raw::scoring::SLOT_ID)?;
        if slot_id < 0 {
            continue;
        }
        let world_position = Some([
            read_f64(bytes, offset + raw::scoring::WORLD_POSITION)?,
            read_f64(bytes, offset + raw::scoring::WORLD_POSITION + 8)?,
            read_f64(bytes, offset + raw::scoring::WORLD_POSITION + 16)?,
        ]);
        field.push(crate::VehicleScoringSnapshot {
            slot_id,
            driver_name: read_string(bytes, offset + raw::scoring::DRIVER_NAME, 32)?,
            vehicle_name: read_string(bytes, offset + raw::scoring::VEHICLE_NAME, 64)?,
            vehicle_class: read_string(bytes, offset + raw::scoring::VEHICLE_CLASS, 32)?,
            place: Some(read_u8(bytes, offset + raw::scoring::PLACE)? as i32)
                .filter(|value| *value > 0),
            lap_number: i32::from(read_i16(bytes, offset + raw::scoring::TOTAL_LAPS)?),
            lap_distance_m: finite(read_f64(bytes, offset + raw::scoring::LAP_DISTANCE)?),
            current_sector: Some(i32::from(read_i8(bytes, offset + raw::scoring::SECTOR)?)),
            last_lap_seconds: valid_time(read_f64(bytes, offset + raw::scoring::LAST_LAP)?),
            best_lap_seconds: valid_time(read_f64(bytes, offset + raw::scoring::BEST_LAP)?),
            gap_to_next_seconds: finite(read_f64(bytes, offset + raw::scoring::TIME_BEHIND_NEXT)?),
            gap_to_leader_seconds: finite(read_f64(
                bytes,
                offset + raw::scoring::TIME_BEHIND_LEADER,
            )?),
            laps_behind_next: Some(read_i32(bytes, offset + raw::scoring::LAPS_BEHIND_NEXT)?),
            laps_behind_leader: Some(read_i32(bytes, offset + raw::scoring::LAPS_BEHIND_LEADER)?),
            in_pits: read_bool(bytes, offset + raw::scoring::IN_PITS)?,
            in_garage: read_bool(bytes, offset + raw::scoring::IN_GARAGE_STALL)?,
            pit_state: Some(read_u8(bytes, offset + raw::scoring::PIT_STATE)? as i32),
            finish_status: Some(read_u8(bytes, offset + raw::scoring::FINISH_STATUS)? as i32),
            flag: Some(read_u8(bytes, offset + raw::scoring::FLAG)? as i32),
            is_player: read_bool(bytes, offset + raw::scoring::IS_PLAYER)?,
            world_position,
        });
    }
    Ok(field)
}

fn field_gaps(
    field: &[crate::VehicleScoringSnapshot],
    player_slot_id: i32,
) -> (Option<f64>, Option<f64>) {
    let Some(player) = field
        .iter()
        .find(|car| car.slot_id == player_slot_id || car.is_player)
    else {
        return (None, None);
    };
    let Some(player_place) = player.place else {
        return (None, None);
    };

    let gap_from_player = |other: &crate::VehicleScoringSnapshot| {
        if other.lap_number != player.lap_number {
            return None;
        }
        other
            .gap_to_leader_seconds
            .zip(player.gap_to_leader_seconds)
            .map(|(other_gap, player_gap)| (other_gap - player_gap).abs())
    };
    let ahead = field
        .iter()
        .filter(|car| car.place == Some(player_place - 1))
        .find_map(gap_from_player);
    let behind = field
        .iter()
        .filter(|car| car.place == Some(player_place + 1))
        .find_map(gap_from_player);
    (ahead, behind)
}

fn finite(value: f64) -> Option<f64> {
    value.is_finite().then_some(value)
}

fn kelvin_to_celsius(value: f64) -> Option<f64> {
    finite(value)
        .filter(|value| *value > 0.0)
        .map(|value| value - 273.15)
}
fn positive(value: f64) -> Option<f64> {
    value.is_finite().then_some(value).filter(|v| *v > 0.0)
}
fn valid_time(value: f64) -> Option<f64> {
    positive(value)
}
fn subtract_sector(total: Option<f64>, first: Option<f64>) -> Option<f64> {
    Some(total? - first?).filter(|value| value.is_finite() && *value >= 0.0)
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct FrameMarker {
    game_version: i32,
    session_time_bits: u64,
    active_vehicles: u8,
    scoring_vehicles: i32,
    player_index: u8,
    player_has_vehicle: bool,
}

fn read_frame_marker(bytes: &[u8]) -> Result<FrameMarker, TelemetryError> {
    Ok(FrameMarker {
        game_version: read_i32(bytes, OFFSET_GAME_VERSION)?,
        session_time_bits: read_u64(bytes, OFFSET_SCORING_CURRENT_ET)?,
        active_vehicles: read_u8(bytes, OFFSET_TELEMETRY_ACTIVE_VEHICLES)?,
        scoring_vehicles: read_i32(bytes, OFFSET_SCORING_NUM_VEHICLES)?,
        player_index: read_u8(bytes, OFFSET_TELEMETRY_PLAYER_INDEX)?,
        player_has_vehicle: read_bool(bytes, OFFSET_TELEMETRY_PLAYER_HAS_VEHICLE)?,
    })
}

/// Detect the one raw layout this parser is compiled for.
///
/// `gameVersion` is a diagnostic value supplied by LMU, not a shared-memory
/// layout revision. An arbitrary numeric range is not compatibility proof.
/// Validate the fixed layout invariants instead; a future incompatible layout
/// must get its own detector and parser.
fn detect_layout(bytes: &[u8], marker: FrameMarker) -> Option<()> {
    if bytes.len() != BUFFER_SIZE
        || marker.game_version <= 0
        || marker.active_vehicles as usize > MAX_VEHICLES
        || marker.scoring_vehicles < 0
        || marker.scoring_vehicles as usize > MAX_VEHICLES
        || (marker.player_has_vehicle && marker.player_index as usize >= MAX_VEHICLES)
    {
        return None;
    }

    Some(())
}

fn read_i32(bytes: &[u8], offset: usize) -> Result<i32, TelemetryError> {
    read_array::<4>(bytes, offset).map(i32::from_le_bytes)
}

fn read_i16(bytes: &[u8], offset: usize) -> Result<i16, TelemetryError> {
    read_array::<2>(bytes, offset).map(i16::from_le_bytes)
}

fn read_u8(bytes: &[u8], offset: usize) -> Result<u8, TelemetryError> {
    read_array::<1>(bytes, offset).map(|bytes| bytes[0])
}

fn read_i8(bytes: &[u8], offset: usize) -> Result<i8, TelemetryError> {
    read_array::<1>(bytes, offset).map(|bytes| bytes[0] as i8)
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, TelemetryError> {
    read_array::<8>(bytes, offset).map(u64::from_le_bytes)
}

fn read_bool(bytes: &[u8], offset: usize) -> Result<bool, TelemetryError> {
    read_u8(bytes, offset).map(|value| value != 0)
}

fn read_f64(bytes: &[u8], offset: usize) -> Result<f64, TelemetryError> {
    read_array::<8>(bytes, offset).map(f64::from_le_bytes)
}

fn read_f32(bytes: &[u8], offset: usize) -> Result<f32, TelemetryError> {
    read_array::<4>(bytes, offset).map(f32::from_le_bytes)
}

fn read_array<const N: usize>(bytes: &[u8], offset: usize) -> Result<[u8; N], TelemetryError> {
    let end = offset
        .checked_add(size_of::<[u8; N]>())
        .ok_or(TelemetryError::BufferTooSmall)?;
    let slice = bytes
        .get(offset..end)
        .ok_or(TelemetryError::BufferTooSmall)?;
    let mut out = [0; N];
    out.copy_from_slice(slice);
    Ok(out)
}

fn read_string(bytes: &[u8], offset: usize, len: usize) -> Result<Option<String>, TelemetryError> {
    let end = offset
        .checked_add(len)
        .ok_or(TelemetryError::BufferTooSmall)?;
    let slice = bytes
        .get(offset..end)
        .ok_or(TelemetryError::BufferTooSmall)?;
    let end = slice
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(slice.len());
    let value = String::from_utf8_lossy(&slice[..end]).trim().to_string();
    Ok((!value.is_empty()).then_some(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_buffer_has_no_sample() {
        let bytes = vec![0; BUFFER_SIZE];
        assert_eq!(read_sample_from_bytes(&bytes).unwrap(), None);
    }

    #[test]
    fn ignores_missing_player_vehicle() {
        let mut bytes = vec![0; BUFFER_SIZE];
        write_i32(&mut bytes, OFFSET_GAME_VERSION, 1);
        write_u8(&mut bytes, OFFSET_TELEMETRY_ACTIVE_VEHICLES, 1);
        write_u8(&mut bytes, OFFSET_TELEMETRY_PLAYER_INDEX, 0);
        write_u8(&mut bytes, OFFSET_TELEMETRY_PLAYER_HAS_VEHICLE, 0);

        assert_eq!(read_sample_from_bytes(&bytes).unwrap(), None);
    }

    #[test]
    fn reads_player_vehicle_sample() {
        let mut bytes = vec![0; BUFFER_SIZE];
        write_i32(&mut bytes, OFFSET_GAME_VERSION, 1);
        write_string(&mut bytes, OFFSET_SCORING_TRACK_NAME, 64, "Sebring");
        write_i32(&mut bytes, OFFSET_SCORING_SESSION, 5);
        write_f64(&mut bytes, OFFSET_SCORING_CURRENT_ET, 12.5);
        write_f64(&mut bytes, OFFSET_TRACK_LENGTH, 5_000.0);
        write_u8(&mut bytes, OFFSET_SCORING_GAME_PHASE, 5);
        write_i32(&mut bytes, OFFSET_SCORING_NUM_VEHICLES, 2);
        write_u8(&mut bytes, OFFSET_TELEMETRY_ACTIVE_VEHICLES, 2);
        write_u8(&mut bytes, OFFSET_TELEMETRY_PLAYER_INDEX, 1);
        write_u8(&mut bytes, OFFSET_TELEMETRY_PLAYER_HAS_VEHICLE, 1);

        let telemetry_offset = OFFSET_TELEMETRY_VEHICLES + VEHICLE_TELEMETRY_SIZE;
        write_i32(&mut bytes, telemetry_offset + OFFSET_TELEMETRY_SLOT_ID, 42);
        write_string(
            &mut bytes,
            telemetry_offset + OFFSET_TELEMETRY_VEHICLE_NAME,
            64,
            "Porsche 963",
        );
        write_i32(&mut bytes, telemetry_offset + OFFSET_LAP_NUMBER, 3);
        write_f64(&mut bytes, telemetry_offset + OFFSET_LAP_START_ET, 8.0);
        write_f64(
            &mut bytes,
            telemetry_offset + raw::telemetry::LOCAL_VELOCITY,
            3.0,
        );
        write_f64(
            &mut bytes,
            telemetry_offset + raw::telemetry::LOCAL_VELOCITY + 8,
            4.0,
        );
        write_i32(&mut bytes, telemetry_offset + OFFSET_GEAR, 4);
        write_f64(&mut bytes, telemetry_offset + OFFSET_RPM, 8_800.0);
        write_f64(&mut bytes, telemetry_offset + OFFSET_THROTTLE, 0.9);
        write_f64(&mut bytes, telemetry_offset + OFFSET_BRAKE, 0.1);
        write_f64(&mut bytes, telemetry_offset + OFFSET_STEERING, -0.2);
        write_f64(&mut bytes, telemetry_offset + OFFSET_CLUTCH, 0.0);
        write_i32(&mut bytes, telemetry_offset + OFFSET_SECTOR, 1);
        write_f64(&mut bytes, telemetry_offset + OFFSET_FUEL, 24.8);
        write_f64(&mut bytes, telemetry_offset + OFFSET_FUEL_CAPACITY, 110.0);
        write_f64(&mut bytes, telemetry_offset + OFFSET_ENGINE_TORQUE, 740.0);
        write_f64(
            &mut bytes,
            telemetry_offset + OFFSET_ENGINE_WATER_TEMP,
            93.0,
        );
        write_f64(&mut bytes, telemetry_offset + OFFSET_ENGINE_OIL_TEMP, 108.0);
        write_f64(&mut bytes, telemetry_offset + OFFSET_REAR_BRAKE_BIAS, 0.4);
        write_u8(&mut bytes, telemetry_offset + OFFSET_LAP_INVALIDATED, 1);
        write_u8(&mut bytes, telemetry_offset + raw::telemetry::DETACHED, 1);
        write_u8(
            &mut bytes,
            telemetry_offset + raw::telemetry::DENT_SEVERITY,
            2,
        );
        write_f64(
            &mut bytes,
            telemetry_offset + raw::telemetry::LAST_IMPACT_MAGNITUDE,
            12.5,
        );
        write_u8(&mut bytes, telemetry_offset + OFFSET_ELECTRIC_STATE, 2);
        write_u8(&mut bytes, telemetry_offset + OFFSET_TC, 3);
        write_u8(&mut bytes, telemetry_offset + OFFSET_ABS, 2);
        write_f64(&mut bytes, telemetry_offset + OFFSET_BATTERY, 0.75);
        write_f32(
            &mut bytes,
            telemetry_offset + raw::telemetry::STATE_OF_CHARGE,
            73.5,
        );

        let wheel_offset = telemetry_offset + OFFSET_WHEELS;
        write_f64(&mut bytes, wheel_offset + raw::wheel::PRESSURE, 182.0);
        write_f64(&mut bytes, wheel_offset + raw::wheel::TEMPERATURE, 373.15);
        write_f64(
            &mut bytes,
            wheel_offset + raw::wheel::TEMPERATURE + 8,
            383.15,
        );
        write_f64(
            &mut bytes,
            wheel_offset + raw::wheel::TEMPERATURE + 16,
            393.15,
        );
        write_f64(&mut bytes, wheel_offset + raw::wheel::BRAKE_TEMP, 620.0);
        write_f64(&mut bytes, wheel_offset + raw::wheel::WEAR, 0.82);
        write_f64(
            &mut bytes,
            wheel_offset + raw::wheel::SUSPENSION_DEFLECTION,
            0.012,
        );
        write_u8(&mut bytes, wheel_offset + raw::wheel::SURFACE_TYPE, 1);

        let scoring_offset = OFFSET_SCORING_VEHICLES;
        write_i32(&mut bytes, scoring_offset + OFFSET_SCORING_SLOT_ID, 42);
        write_u8(&mut bytes, scoring_offset + OFFSET_SCORING_SECTOR, 2);
        write_u8(&mut bytes, scoring_offset + OFFSET_SCORING_IS_PLAYER, 1);
        write_u8(&mut bytes, scoring_offset + OFFSET_SCORING_IN_PITS, 1);
        write_u8(&mut bytes, scoring_offset + raw::scoring::PLACE, 5);
        write_u8(&mut bytes, scoring_offset + raw::scoring::FLAG, 6);
        write_u8(&mut bytes, scoring_offset + raw::scoring::COUNT_LAP_FLAG, 2);
        write_u8(&mut bytes, scoring_offset + raw::scoring::PIT_STATE, 3);
        write_f64(
            &mut bytes,
            scoring_offset + raw::scoring::TIME_BEHIND_NEXT,
            1.25,
        );
        write_string(
            &mut bytes,
            scoring_offset + OFFSET_SCORING_VEHICLE_NAME,
            64,
            "Porsche 963 Scoring",
        );
        write_string(
            &mut bytes,
            scoring_offset + OFFSET_SCORING_VEHICLE_CLASS,
            32,
            "Hypercar",
        );
        write_f64(
            &mut bytes,
            scoring_offset + OFFSET_SCORING_LAP_DISTANCE,
            1_250.0,
        );
        write_f64(
            &mut bytes,
            scoring_offset + raw::scoring::CURRENT_SECTOR1,
            31.0,
        );
        write_f64(
            &mut bytes,
            scoring_offset + raw::scoring::CURRENT_SECTOR2,
            63.0,
        );
        write_f64(
            &mut bytes,
            scoring_offset + raw::scoring::LAST_SECTOR1,
            30.0,
        );
        write_f64(
            &mut bytes,
            scoring_offset + raw::scoring::LAST_SECTOR2,
            61.0,
        );
        write_f64(&mut bytes, scoring_offset + raw::scoring::LAST_LAP, 92.0);
        write_f64(
            &mut bytes,
            scoring_offset + raw::scoring::BEST_SECTOR1,
            29.0,
        );
        write_f64(
            &mut bytes,
            scoring_offset + raw::scoring::BEST_SECTOR2,
            60.0,
        );
        write_f64(&mut bytes, scoring_offset + raw::scoring::BEST_LAP, 90.0);

        let sample = read_sample_from_bytes(&bytes).unwrap().unwrap();

        assert_eq!(sample.gear, Gear::Forward(4));
        assert_eq!(sample.lap_number, 3);
        assert_eq!(sample.speed_mps, 5.0);
        assert_eq!(sample.lap_distance_m, Some(1_250.0));
        assert_eq!(sample.track_length_m, Some(5_000.0));
        assert_eq!(sample.lap_progress(), Some(0.25));
        assert_eq!(sample.sector, 2);
        assert_eq!(sample.metadata.track_name.as_deref(), Some("Sebring"));
        assert_eq!(
            sample.metadata.vehicle_name.as_deref(),
            Some("Porsche 963 Scoring")
        );
        assert_eq!(sample.metadata.vehicle_class.as_deref(), Some("Hypercar"));
        assert_eq!(sample.session.position, Some(5));
        assert_eq!(sample.session.flag, Some(6));
        assert_eq!(sample.session.count_lap_flag, Some(2));
        assert_eq!(sample.session.pit_state, Some(3));
        assert_eq!(sample.session.gap_ahead_seconds, Some(1.25));
        assert_eq!(sample.vehicle.fuel_liters, Some(24.8));
        assert_eq!(sample.vehicle.fuel_capacity_liters, Some(110.0));
        assert_eq!(sample.vehicle.engine_water_temp_c, Some(93.0));
        assert_eq!(sample.vehicle.engine_torque_nm, Some(740.0));
        assert_eq!(sample.vehicle.body_detached, Some(true));
        assert_eq!(sample.vehicle.dent_severity[0], Some(2));
        assert_eq!(sample.vehicle.last_impact_magnitude, Some(12.5));
        assert_eq!(sample.vehicle.brake_bias_front_percent, Some(60.0));
        assert_eq!(sample.vehicle.tc_setting, Some(3));
        assert_eq!(sample.vehicle.abs_setting, Some(2));
        assert_eq!(sample.vehicle.battery_charge_percent, Some(75.0));
        assert_eq!(sample.vehicle.state_of_charge_percent, Some(73.5));
        assert_eq!(sample.metadata.lap_invalidated, Some(true));
        assert_eq!(sample.wheels.front_left.pressure_kpa, Some(182.0));
        assert_eq!(sample.wheels.front_left.surface_temp_center_c, Some(110.0));
        assert_eq!(sample.wheels.front_left.brake_temp_c, Some(620.0));
        assert_eq!(sample.wheels.front_left.wear_percent, Some(82.0));
        assert_eq!(
            sample.wheels.front_left.suspension_deflection_m,
            Some(0.012)
        );
        assert_eq!(sample.wheels.front_left.surface_type, Some(1));
        assert_eq!(sample.sector_times.current_sector1_seconds, Some(31.0));
        assert_eq!(sample.sector_times.current_sector2_seconds, Some(32.0));
        assert_eq!(sample.sector_times.last_sector3_seconds, Some(31.0));
        assert_eq!(sample.sector_times.best_sector3_seconds, Some(30.0));
        assert_eq!(sample.metadata.session_kind, SessionKind::Qualifying);
        assert_eq!(sample.metadata.game_phase, GamePhase::GreenFlag);
        assert!(sample.metadata.in_pits);
        assert!(!sample.metadata.in_garage);
        assert_eq!(sample.metadata.player_slot_id, 42);
    }

    #[test]
    fn ignores_invalid_layout_marker() {
        let mut bytes = vec![0; BUFFER_SIZE];
        write_i32(&mut bytes, OFFSET_GAME_VERSION, 1);
        write_u8(&mut bytes, OFFSET_TELEMETRY_ACTIVE_VEHICLES, 255);

        assert_eq!(read_sample_from_bytes(&bytes).unwrap(), None);
    }

    #[test]
    fn detects_torn_frame_markers() {
        let before = FrameMarker {
            game_version: 1,
            session_time_bits: 10.0f64.to_bits(),
            active_vehicles: 1,
            scoring_vehicles: 1,
            player_index: 0,
            player_has_vehicle: true,
        };
        let after = FrameMarker {
            session_time_bits: 10.1f64.to_bits(),
            ..before
        };

        assert!(matches!(
            ensure_same_frame(before, after),
            Err(TelemetryError::TornFrame)
        ));
    }

    #[test]
    fn treats_zero_kelvin_wheel_temperature_as_unavailable() {
        assert_eq!(kelvin_to_celsius(0.0), None);
        assert_eq!(kelvin_to_celsius(f64::NAN), None);
        assert_eq!(kelvin_to_celsius(373.15), Some(100.0));
    }

    #[test]
    fn derives_gaps_from_adjacent_same_lap_scoring_rows() {
        let make_car = |slot_id, place, lap, leader_gap, is_player| crate::VehicleScoringSnapshot {
            slot_id,
            driver_name: None,
            vehicle_name: None,
            vehicle_class: None,
            place: Some(place),
            lap_number: lap,
            lap_distance_m: None,
            current_sector: None,
            last_lap_seconds: None,
            best_lap_seconds: None,
            gap_to_next_seconds: None,
            gap_to_leader_seconds: Some(leader_gap),
            laps_behind_next: None,
            laps_behind_leader: None,
            in_pits: false,
            in_garage: false,
            pit_state: None,
            finish_status: None,
            flag: None,
            is_player,
            world_position: None,
        };
        let field = vec![
            make_car(10, 4, 12, 4.0, false),
            make_car(42, 5, 12, 5.5, true),
            make_car(11, 6, 12, 6.25, false),
        ];

        assert_eq!(field_gaps(&field, 42), (Some(1.5), Some(0.75)));
    }

    fn write_i32(bytes: &mut [u8], offset: usize, value: i32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u8(bytes: &mut [u8], offset: usize, value: u8) {
        bytes[offset] = value;
    }

    fn write_f64(bytes: &mut [u8], offset: usize, value: f64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    fn write_f32(bytes: &mut [u8], offset: usize, value: f32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn write_string(bytes: &mut [u8], offset: usize, len: usize, value: &str) {
        let target = &mut bytes[offset..offset + len];
        target.fill(0);
        let value = value.as_bytes();
        let len = value.len().min(target.len().saturating_sub(1));
        target[..len].copy_from_slice(&value[..len]);
    }
}

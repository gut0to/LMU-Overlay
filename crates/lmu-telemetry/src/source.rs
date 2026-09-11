use std::{error::Error, fmt, mem::size_of};

use crate::{GamePhase, Gear, SessionKind, TelemetryMetadata, TelemetrySample};

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
const OFFSET_LOCAL_VEL: usize = 184;
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
const EXPECTED_GAME_VERSION_MIN: i32 = 1;
const EXPECTED_GAME_VERSION_MAX: i32 = 99_999;

pub struct SharedMemoryTelemetrySource {
    inner: PlatformTelemetrySource,
}

impl SharedMemoryTelemetrySource {
    pub fn open() -> Result<Self, TelemetryError> {
        Ok(Self {
            inner: PlatformTelemetrySource::open()?,
        })
    }
}

impl TelemetrySource for SharedMemoryTelemetrySource {
    fn is_available(&self) -> bool {
        self.inner.is_available()
    }

    fn read_sample(&mut self) -> Result<Option<TelemetrySample>, TelemetryError> {
        if !self.inner.is_available() {
            self.inner = PlatformTelemetrySource::open()?;
        }

        self.inner.read_sample()
    }
}

#[cfg(windows)]
struct PlatformTelemetrySource {
    handle: windows_sys::Win32::Foundation::HANDLE,
    view: windows_sys::Win32::System::Memory::MEMORY_MAPPED_VIEW_ADDRESS,
}

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
            });
        }

        let view = unsafe { MapViewOfFile(handle, FILE_MAP_READ, 0, 0, BUFFER_SIZE) };
        if view.Value.is_null() {
            unsafe {
                windows_sys::Win32::Foundation::CloseHandle(handle);
            }
            return Err(TelemetryError::MappingFailed);
        }

        Ok(Self { handle, view })
    }

    fn is_available(&self) -> bool {
        !self.handle.is_null() && !self.view.Value.is_null()
    }

    fn read_sample(&mut self) -> Result<Option<TelemetrySample>, TelemetryError> {
        if !self.is_available() {
            return Ok(None);
        }

        let bytes = unsafe { slice::from_raw_parts(self.view.Value.cast::<u8>(), BUFFER_SIZE) };
        read_consistent_sample_from_bytes(bytes)
            .map(|sample| sample.map(TelemetrySample::sanitized))
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
) -> Result<Option<TelemetrySample>, TelemetryError> {
    let mut last_error = None;
    for _ in 0..MAX_TORN_FRAME_RETRIES {
        match read_sample_once(bytes) {
            Ok(sample) => return Ok(sample),
            Err(TelemetryError::TornFrame) => last_error = Some(TelemetryError::TornFrame),
            Err(error) => return Err(error),
        }
    }

    Err(last_error.unwrap_or(TelemetryError::TornFrame))
}

#[cfg(test)]
fn read_sample_from_bytes(bytes: &[u8]) -> Result<Option<TelemetrySample>, TelemetryError> {
    read_consistent_sample_from_bytes(bytes)
}

fn read_sample_once(bytes: &[u8]) -> Result<Option<TelemetrySample>, TelemetryError> {
    if bytes.len() < BUFFER_SIZE {
        return Err(TelemetryError::BufferTooSmall);
    }

    let marker_before = read_frame_marker(bytes)?;
    if !is_supported_game_version(marker_before.game_version) {
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

    let sample = TelemetrySample {
        timestamp_seconds: read_f64(bytes, OFFSET_SCORING_CURRENT_ET)
            .or_else(|_| read_f64(bytes, vehicle_offset + OFFSET_ELAPSED_TIME))?,
        speed_mps: read_f64(bytes, vehicle_offset + OFFSET_LOCAL_VEL + 8)?,
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
        metadata: read_metadata(bytes, scoring_offset, vehicle_offset, player_slot_id)?,
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

fn is_supported_game_version(version: i32) -> bool {
    (EXPECTED_GAME_VERSION_MIN..=EXPECTED_GAME_VERSION_MAX).contains(&version)
}

fn read_i32(bytes: &[u8], offset: usize) -> Result<i32, TelemetryError> {
    read_array::<4>(bytes, offset).map(i32::from_le_bytes)
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
        write_f64(&mut bytes, telemetry_offset + OFFSET_LOCAL_VEL + 8, 72.0);
        write_i32(&mut bytes, telemetry_offset + OFFSET_GEAR, 4);
        write_f64(&mut bytes, telemetry_offset + OFFSET_RPM, 8_800.0);
        write_f64(&mut bytes, telemetry_offset + OFFSET_THROTTLE, 0.9);
        write_f64(&mut bytes, telemetry_offset + OFFSET_BRAKE, 0.1);
        write_f64(&mut bytes, telemetry_offset + OFFSET_STEERING, -0.2);
        write_f64(&mut bytes, telemetry_offset + OFFSET_CLUTCH, 0.0);
        write_i32(&mut bytes, telemetry_offset + OFFSET_SECTOR, 1);

        let scoring_offset = OFFSET_SCORING_VEHICLES;
        write_i32(&mut bytes, scoring_offset + OFFSET_SCORING_SLOT_ID, 42);
        write_u8(&mut bytes, scoring_offset + OFFSET_SCORING_SECTOR, 2);
        write_u8(&mut bytes, scoring_offset + OFFSET_SCORING_IS_PLAYER, 1);
        write_u8(&mut bytes, scoring_offset + OFFSET_SCORING_IN_PITS, 1);
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

        let sample = read_sample_from_bytes(&bytes).unwrap().unwrap();

        assert_eq!(sample.gear, Gear::Forward(4));
        assert_eq!(sample.lap_number, 3);
        assert_eq!(sample.speed_mps, 72.0);
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
        assert_eq!(sample.metadata.session_kind, SessionKind::Qualifying);
        assert_eq!(sample.metadata.game_phase, GamePhase::GreenFlag);
        assert!(sample.metadata.in_pits);
        assert!(!sample.metadata.in_garage);
        assert_eq!(sample.metadata.player_slot_id, 42);
    }

    #[test]
    fn ignores_unsupported_game_version() {
        let mut bytes = vec![0; BUFFER_SIZE];
        write_i32(&mut bytes, OFFSET_GAME_VERSION, 1_000_000);

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

    fn write_i32(bytes: &mut [u8], offset: usize, value: i32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u8(bytes: &mut [u8], offset: usize, value: u8) {
        bytes[offset] = value;
    }

    fn write_f64(bytes: &mut [u8], offset: usize, value: f64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    fn write_string(bytes: &mut [u8], offset: usize, len: usize, value: &str) {
        let target = &mut bytes[offset..offset + len];
        target.fill(0);
        let value = value.as_bytes();
        let len = value.len().min(target.len().saturating_sub(1));
        target[..len].copy_from_slice(&value[..len]);
    }
}

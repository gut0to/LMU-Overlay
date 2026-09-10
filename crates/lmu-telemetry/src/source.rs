use std::{error::Error, fmt, mem::size_of};

use crate::{Gear, TelemetrySample};

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
const OFFSET_TELEMETRY_ACTIVE_VEHICLES: usize = OFFSET_TELEMETRY_DATA;
const OFFSET_TELEMETRY_PLAYER_INDEX: usize = OFFSET_TELEMETRY_DATA + 1;
const OFFSET_TELEMETRY_PLAYER_HAS_VEHICLE: usize = OFFSET_TELEMETRY_DATA + 2;
const OFFSET_SCORING_VEHICLES: usize = OFFSET_SCORING_DATA + SCORING_INFO_SIZE + 12;
const OFFSET_TELEMETRY_VEHICLES: usize = OFFSET_TELEMETRY_DATA + 4;

const OFFSET_ELAPSED_TIME: usize = 12;
const OFFSET_LAP_NUMBER: usize = 20;
const OFFSET_LAP_START_ET: usize = 24;
const OFFSET_LOCAL_VEL: usize = 184;
const OFFSET_GEAR: usize = 352;
const OFFSET_RPM: usize = 356;
const OFFSET_THROTTLE: usize = 388;
const OFFSET_BRAKE: usize = 396;
const OFFSET_STEERING: usize = 404;
const OFFSET_CLUTCH: usize = 412;
const OFFSET_SECTOR: usize = 600;

const OFFSET_SCORING_LAP_DISTANCE: usize = 104;

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
        read_sample_from_bytes(bytes).map(|sample| sample.map(TelemetrySample::sanitized))
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

fn read_sample_from_bytes(bytes: &[u8]) -> Result<Option<TelemetrySample>, TelemetryError> {
    if bytes.len() < BUFFER_SIZE {
        return Err(TelemetryError::BufferTooSmall);
    }

    if read_i32(bytes, OFFSET_GAME_VERSION)? == 0 {
        return Ok(None);
    }

    let active_vehicles = read_u8(bytes, OFFSET_TELEMETRY_ACTIVE_VEHICLES)? as usize;
    let scoring_vehicles =
        read_i32(bytes, OFFSET_SCORING_NUM_VEHICLES)?.clamp(0, MAX_VEHICLES as i32) as usize;
    let player_index = read_u8(bytes, OFFSET_TELEMETRY_PLAYER_INDEX)? as usize;
    let player_has_vehicle = read_bool(bytes, OFFSET_TELEMETRY_PLAYER_HAS_VEHICLE)?;

    if !player_has_vehicle || player_index >= active_vehicles || player_index >= MAX_VEHICLES {
        return Ok(None);
    }

    let vehicle_offset = OFFSET_TELEMETRY_VEHICLES + player_index * VEHICLE_TELEMETRY_SIZE;
    let scoring_offset = (player_index < scoring_vehicles)
        .then_some(OFFSET_SCORING_VEHICLES + player_index * VEHICLE_SCORING_SIZE);

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
        sector: read_i32(bytes, vehicle_offset + OFFSET_SECTOR)?,
    };

    Ok(Some(sample.sanitized()))
}

fn read_i32(bytes: &[u8], offset: usize) -> Result<i32, TelemetryError> {
    read_array::<4>(bytes, offset).map(i32::from_le_bytes)
}

fn read_u8(bytes: &[u8], offset: usize) -> Result<u8, TelemetryError> {
    read_array::<1>(bytes, offset).map(|bytes| bytes[0])
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
        write_f64(&mut bytes, OFFSET_SCORING_CURRENT_ET, 12.5);
        write_f64(&mut bytes, OFFSET_TRACK_LENGTH, 5_000.0);
        write_i32(&mut bytes, OFFSET_SCORING_NUM_VEHICLES, 2);
        write_u8(&mut bytes, OFFSET_TELEMETRY_ACTIVE_VEHICLES, 2);
        write_u8(&mut bytes, OFFSET_TELEMETRY_PLAYER_INDEX, 1);
        write_u8(&mut bytes, OFFSET_TELEMETRY_PLAYER_HAS_VEHICLE, 1);

        let telemetry_offset = OFFSET_TELEMETRY_VEHICLES + VEHICLE_TELEMETRY_SIZE;
        write_i32(&mut bytes, telemetry_offset + OFFSET_ID, 7);
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

        let scoring_offset = OFFSET_SCORING_VEHICLES + VEHICLE_SCORING_SIZE;
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
        assert_eq!(sample.sector, 1);
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
}

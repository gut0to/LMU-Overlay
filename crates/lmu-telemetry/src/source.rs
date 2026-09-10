use std::{error::Error, ffi::OsStr, fmt, mem::size_of, ptr, slice};

use crate::{Gear, TelemetrySample};

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
            TelemetryError::UnsupportedPlatform => f.write_str("shared memory reading is only supported on Windows"),
            TelemetryError::MappingFailed => f.write_str("could not map the telemetry shared memory buffer"),
            TelemetryError::BufferTooSmall => f.write_str("telemetry shared memory buffer is smaller than expected"),
            TelemetryError::TornFrame => f.write_str("telemetry frame changed while it was being read"),
        }
    }
}

impl Error for TelemetryError {}

const TELEMETRY_MAP_NAME: &str = "$rFactor2SMMP_Telemetry$";
const VERSION_BLOCK_SIZE: usize = 8;
const TELEMETRY_HEADER_SIZE: usize = 8;
const VEHICLE_SIZE: usize = 1_368;
const MAX_VEHICLES: usize = 128;
const BUFFER_SIZE: usize = VERSION_BLOCK_SIZE + TELEMETRY_HEADER_SIZE + VEHICLE_SIZE * MAX_VEHICLES;

const VEHICLE0_OFFSET: usize = VERSION_BLOCK_SIZE + TELEMETRY_HEADER_SIZE;
const OFFSET_ID: usize = 0;
const OFFSET_ELAPSED_TIME: usize = 16;
const OFFSET_LAP_NUMBER: usize = 24;
const OFFSET_LAP_START_ET: usize = 28;
const OFFSET_LOCAL_VEL: usize = 200;
const OFFSET_GEAR: usize = 344;
const OFFSET_RPM: usize = 348;
const OFFSET_THROTTLE: usize = 388;
const OFFSET_BRAKE: usize = 396;
const OFFSET_STEERING: usize = 404;
const OFFSET_CLUTCH: usize = 412;
const OFFSET_SECTOR: usize = 500;

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
        self.inner.read_sample()
    }
}

#[cfg(windows)]
struct PlatformTelemetrySource {
    handle: windows_sys::Win32::Foundation::HANDLE,
    view: *const u8,
}

#[cfg(windows)]
impl PlatformTelemetrySource {
    fn open() -> Result<Self, TelemetryError> {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::System::Memory::{
            MapViewOfFile, OpenFileMappingW, FILE_MAP_READ,
        };

        let mut wide_name: Vec<u16> = OsStr::new(TELEMETRY_MAP_NAME)
            .encode_wide()
            .chain([0])
            .collect();
        let handle = unsafe { OpenFileMappingW(FILE_MAP_READ, 0, wide_name.as_mut_ptr()) };

        if handle == 0 {
            return Ok(Self {
                handle,
                view: ptr::null(),
            });
        }

        let view = unsafe { MapViewOfFile(handle, FILE_MAP_READ, 0, 0, BUFFER_SIZE) } as *const u8;
        if view.is_null() {
            unsafe {
                windows_sys::Win32::Foundation::CloseHandle(handle);
            }
            return Err(TelemetryError::MappingFailed);
        }

        Ok(Self { handle, view })
    }

    fn is_available(&self) -> bool {
        self.handle != 0 && !self.view.is_null()
    }

    fn read_sample(&mut self) -> Result<Option<TelemetrySample>, TelemetryError> {
        if !self.is_available() {
            return Ok(None);
        }

        let bytes = unsafe { slice::from_raw_parts(self.view, BUFFER_SIZE) };
        read_sample_from_bytes(bytes)
    }
}

#[cfg(windows)]
impl Drop for PlatformTelemetrySource {
    fn drop(&mut self) {
        if !self.view.is_null() {
            unsafe {
                windows_sys::Win32::System::Memory::UnmapViewOfFile(self.view as _);
            }
        }
        if self.handle != 0 {
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

    let begin = read_u32(bytes, 0)?;
    let end_before = read_u32(bytes, 4)?;
    if begin != end_before {
        return Err(TelemetryError::TornFrame);
    }

    let num_vehicles = read_i32(bytes, VERSION_BLOCK_SIZE + 4)?.clamp(0, MAX_VEHICLES as i32);
    if num_vehicles == 0 {
        return Ok(None);
    }

    let vehicle_offset = (0..num_vehicles as usize)
        .map(|index| VEHICLE0_OFFSET + index * VEHICLE_SIZE)
        .find(|offset| read_i32(bytes, offset + OFFSET_ID).unwrap_or(-1) >= 0)
        .ok_or(TelemetryError::BufferTooSmall)?;

    let end_after = read_u32(bytes, 4)?;
    if begin != end_after {
        return Err(TelemetryError::TornFrame);
    }

    Ok(Some(TelemetrySample {
        timestamp_seconds: read_f64(bytes, vehicle_offset + OFFSET_ELAPSED_TIME)?,
        speed_mps: read_f64(bytes, vehicle_offset + OFFSET_LOCAL_VEL + 8)?,
        rpm: read_f64(bytes, vehicle_offset + OFFSET_RPM)?,
        gear: Gear::from(read_i32(bytes, vehicle_offset + OFFSET_GEAR)?),
        throttle: read_f64(bytes, vehicle_offset + OFFSET_THROTTLE)?,
        brake: read_f64(bytes, vehicle_offset + OFFSET_BRAKE)?,
        clutch: read_f64(bytes, vehicle_offset + OFFSET_CLUTCH)?,
        steering: read_f64(bytes, vehicle_offset + OFFSET_STEERING)?,
        lap_number: read_i32(bytes, vehicle_offset + OFFSET_LAP_NUMBER)?,
        lap_start_seconds: read_f64(bytes, vehicle_offset + OFFSET_LAP_START_ET)?,
        sector: read_i32(bytes, vehicle_offset + OFFSET_SECTOR)?,
    }))
}

fn read_i32(bytes: &[u8], offset: usize) -> Result<i32, TelemetryError> {
    read_array::<4>(bytes, offset).map(i32::from_le_bytes)
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, TelemetryError> {
    read_array::<4>(bytes, offset).map(u32::from_le_bytes)
}

fn read_f64(bytes: &[u8], offset: usize) -> Result<f64, TelemetryError> {
    read_array::<8>(bytes, offset).map(f64::from_le_bytes)
}

fn read_array<const N: usize>(bytes: &[u8], offset: usize) -> Result<[u8; N], TelemetryError> {
    let end = offset.checked_add(size_of::<[u8; N]>()).ok_or(TelemetryError::BufferTooSmall)?;
    let slice = bytes.get(offset..end).ok_or(TelemetryError::BufferTooSmall)?;
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
    fn detects_torn_frame() {
        let mut bytes = vec![0; BUFFER_SIZE];
        bytes[0..4].copy_from_slice(&1_u32.to_le_bytes());
        bytes[4..8].copy_from_slice(&2_u32.to_le_bytes());

        assert!(matches!(
            read_sample_from_bytes(&bytes),
            Err(TelemetryError::TornFrame)
        ));
    }

    #[test]
    fn reads_first_vehicle_sample() {
        let mut bytes = vec![0; BUFFER_SIZE];
        write_u32(&mut bytes, 0, 4);
        write_u32(&mut bytes, 4, 4);
        write_i32(&mut bytes, VERSION_BLOCK_SIZE + 4, 1);

        let offset = VEHICLE0_OFFSET;
        write_i32(&mut bytes, offset + OFFSET_ID, 7);
        write_f64(&mut bytes, offset + OFFSET_ELAPSED_TIME, 12.5);
        write_i32(&mut bytes, offset + OFFSET_LAP_NUMBER, 3);
        write_f64(&mut bytes, offset + OFFSET_LAP_START_ET, 8.0);
        write_f64(&mut bytes, offset + OFFSET_LOCAL_VEL + 8, 72.0);
        write_i32(&mut bytes, offset + OFFSET_GEAR, 4);
        write_f64(&mut bytes, offset + OFFSET_RPM, 8_800.0);
        write_f64(&mut bytes, offset + OFFSET_THROTTLE, 0.9);
        write_f64(&mut bytes, offset + OFFSET_BRAKE, 0.1);
        write_f64(&mut bytes, offset + OFFSET_STEERING, -0.2);
        write_f64(&mut bytes, offset + OFFSET_CLUTCH, 0.0);
        write_i32(&mut bytes, offset + OFFSET_SECTOR, 1);

        let sample = read_sample_from_bytes(&bytes).unwrap().unwrap();

        assert_eq!(sample.gear, Gear::Forward(4));
        assert_eq!(sample.lap_number, 3);
        assert_eq!(sample.speed_mps, 72.0);
        assert_eq!(sample.sector, 1);
    }

    fn write_i32(bytes: &mut [u8], offset: usize, value: i32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn write_f64(bytes: &mut [u8], offset: usize, value: f64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }
}


use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

use lap_engine::{ReferenceLap, ReferencePoint};

const MAGIC: &[u8; 8] = b"HOLAP001";
const POINT_SIZE: usize = 52;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceLapKey {
    pub track: String,
    pub car: String,
    pub layout: String,
}

impl ReferenceLapKey {
    pub fn fallback() -> Self {
        Self {
            track: "unknown-track".to_string(),
            car: "unknown-car".to_string(),
            layout: "default".to_string(),
        }
    }

    fn file_stem(&self) -> String {
        format!(
            "{}__{}__{}",
            sanitize_path_part(&self.track),
            sanitize_path_part(&self.car),
            sanitize_path_part(&self.layout)
        )
    }
}

#[derive(Debug, Clone)]
pub struct ReferenceLapStore {
    root: PathBuf,
}

impl ReferenceLapStore {
    pub fn appdata() -> Self {
        let root = env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("HashOverlay")
            .join("laps");
        Self { root }
    }

    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn load_personal_best(
        &self,
        key: &ReferenceLapKey,
    ) -> Result<Option<ReferenceLap>, StorageError> {
        let path = self.personal_best_path(key);
        if !path.exists() {
            return Ok(None);
        }

        read_reference_lap(&path).map(Some)
    }

    pub fn save_personal_best(
        &self,
        key: &ReferenceLapKey,
        lap: &ReferenceLap,
    ) -> Result<(), StorageError> {
        fs::create_dir_all(&self.root)?;
        write_reference_lap(&self.personal_best_path(key), lap)
    }

    pub fn personal_best_path(&self, key: &ReferenceLapKey) -> PathBuf {
        self.root.join(format!("{}.pb-lap", key.file_stem()))
    }
}

#[derive(Debug)]
pub enum StorageError {
    Io(io::Error),
    InvalidFormat,
}

impl From<io::Error> for StorageError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "could not access lap storage: {error}"),
            Self::InvalidFormat => f.write_str("reference lap file has an invalid format"),
        }
    }
}

impl std::error::Error for StorageError {}

fn write_reference_lap(path: &Path, lap: &ReferenceLap) -> Result<(), StorageError> {
    let mut bytes = Vec::with_capacity(16 + lap.points.len() * POINT_SIZE);
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&lap.total_time_seconds.to_le_bytes());
    bytes.extend_from_slice(&(lap.points.len() as u32).to_le_bytes());

    for point in &lap.points {
        bytes.extend_from_slice(&point.progress.to_le_bytes());
        bytes.extend_from_slice(&point.time_seconds.to_le_bytes());
        bytes.extend_from_slice(&point.throttle.to_le_bytes());
        bytes.extend_from_slice(&point.brake.to_le_bytes());
        bytes.extend_from_slice(&point.speed_kph.to_le_bytes());
        bytes.extend_from_slice(&point.gear.to_le_bytes());
        bytes.extend_from_slice(&point.steering.to_le_bytes());
    }

    fs::write(path, bytes)?;
    Ok(())
}

fn read_reference_lap(path: &Path) -> Result<ReferenceLap, StorageError> {
    let bytes = fs::read(path)?;
    if bytes.len() < 20 || bytes.get(..8) != Some(MAGIC) {
        return Err(StorageError::InvalidFormat);
    }

    let total_time_seconds = read_f64(&bytes, 8)?;
    let point_count = read_u32(&bytes, 16)? as usize;
    let expected_len = 20 + point_count * POINT_SIZE;
    if bytes.len() != expected_len {
        return Err(StorageError::InvalidFormat);
    }

    let mut points = Vec::with_capacity(point_count);
    let mut offset = 20;
    for _ in 0..point_count {
        points.push(ReferencePoint {
            progress: read_f64(&bytes, offset)?,
            time_seconds: read_f64(&bytes, offset + 8)?,
            throttle: read_f64(&bytes, offset + 16)?,
            brake: read_f64(&bytes, offset + 24)?,
            speed_kph: read_f64(&bytes, offset + 32)?,
            gear: read_i32(&bytes, offset + 40)?,
            steering: read_f64(&bytes, offset + 44)?,
        });
        offset += POINT_SIZE;
    }

    ReferenceLap::new(total_time_seconds, points).ok_or(StorageError::InvalidFormat)
}

fn read_f64(bytes: &[u8], offset: usize) -> Result<f64, StorageError> {
    read_array::<8>(bytes, offset).map(f64::from_le_bytes)
}

fn read_i32(bytes: &[u8], offset: usize) -> Result<i32, StorageError> {
    read_array::<4>(bytes, offset).map(i32::from_le_bytes)
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, StorageError> {
    read_array::<4>(bytes, offset).map(u32::from_le_bytes)
}

fn read_array<const N: usize>(bytes: &[u8], offset: usize) -> Result<[u8; N], StorageError> {
    let end = offset.checked_add(N).ok_or(StorageError::InvalidFormat)?;
    let slice = bytes.get(offset..end).ok_or(StorageError::InvalidFormat)?;
    let mut out = [0; N];
    out.copy_from_slice(slice);
    Ok(out)
}

fn sanitize_path_part(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|character| match character {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => character,
            _ => '-',
        })
        .collect();

    sanitized.trim_matches('-').to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_and_loads_reference_lap() {
        let root = env::temp_dir().join(format!("hashoverlay-storage-test-{}", std::process::id()));
        let store = ReferenceLapStore::new(&root);
        let key = ReferenceLapKey::fallback();
        let lap = ReferenceLap::new(
            90.0,
            vec![point(0.0, 0.0), point(0.5, 45.0), point(1.0, 90.0)],
        )
        .unwrap();

        store.save_personal_best(&key, &lap).unwrap();
        let loaded = store.load_personal_best(&key).unwrap().unwrap();

        assert_eq!(loaded.total_time_seconds, 90.0);
        assert_eq!(loaded.points.len(), 3);
    }

    #[test]
    fn sanitizes_lap_keys_for_paths() {
        let key = ReferenceLapKey {
            track: "Le Mans/24h".to_string(),
            car: "Car:Hyper".to_string(),
            layout: "2026".to_string(),
        };

        assert_eq!(key.file_stem(), "le-mans-24h__car-hyper__2026");
    }

    fn point(progress: f64, time_seconds: f64) -> ReferencePoint {
        ReferencePoint {
            progress,
            time_seconds,
            throttle: progress,
            brake: 0.0,
            speed_kph: 200.0,
            gear: 5,
            steering: 0.0,
        }
    }
}

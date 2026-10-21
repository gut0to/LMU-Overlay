use std::{
    env,
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;

use lap_engine::{ReferenceLap, ReferencePoint};

#[cfg(windows)]
use windows_sys::Win32::Storage::FileSystem::{
    MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
};

const MAGIC: &[u8; 8] = b"HOLAP001";
const MAGIC_V2: &[u8; 8] = b"HOLAP002";
const SUPPORTED_MAGIC: [&[u8; 8]; 2] = [MAGIC, MAGIC_V2];
const POINT_SIZE: usize = 52;
const MAX_STORED_POINTS: usize = 2_001;
const V2_HEADER_SIZE: usize = 36;
const MAX_REFERENCE_BYTES: usize = V2_HEADER_SIZE + MAX_STORED_POINTS * POINT_SIZE;
static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceLapKey {
    pub track: String,
    pub track_layout: String,
    pub car: String,
    pub legacy_vehicle_class: Option<String>,
}

impl ReferenceLapKey {
    pub fn fallback() -> Self {
        Self {
            track: "unknown-track".to_string(),
            track_layout: String::new(),
            car: "unknown-car".to_string(),
            legacy_vehicle_class: None,
        }
    }

    fn file_stem(&self) -> String {
        let track = sanitize_path_part(&self.track);
        let car = sanitize_path_part(&self.car);
        if self.has_layout() {
            format!("{track}__{}__{car}", sanitize_path_part(&self.track_layout))
        } else {
            format!("{track}__{car}")
        }
    }

    fn has_layout(&self) -> bool {
        !self.track_layout.trim().is_empty() && self.track_layout != "unknown-layout"
    }

    fn legacy_layout_file_stem(&self) -> Option<String> {
        (!self.has_layout()).then(|| {
            format!(
                "{}__unknown-layout__{}",
                sanitize_path_part(&self.track),
                sanitize_path_part(&self.car)
            )
        })
    }

    fn legacy_file_stem(&self) -> Option<String> {
        let vehicle_class = self.legacy_vehicle_class.as_deref()?;
        Some(format!(
            "{}__{}__{}",
            sanitize_path_part(&self.track),
            sanitize_path_part(&self.car),
            sanitize_path_part(vehicle_class)
        ))
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
        if path.exists() {
            return read_reference_lap(&path).map(Some);
        }

        if let Some(legacy_path) = self.legacy_personal_best_path(key) {
            if legacy_path.exists() {
                return read_reference_lap(&legacy_path).map(Some);
            }
        }

        if let Some(file_stem) = key.legacy_layout_file_stem() {
            let legacy_layout_path = self.root.join(format!("{file_stem}.pb-lap"));
            if legacy_layout_path.exists() {
                return read_reference_lap(&legacy_layout_path).map(Some);
            }
        }

        Ok(None)
    }

    pub fn save_personal_best(
        &self,
        key: &ReferenceLapKey,
        lap: &ReferenceLap,
    ) -> Result<(), StorageError> {
        validate_reference_lap(lap)?;
        fs::create_dir_all(&self.root)?;
        write_reference_lap(&self.personal_best_path(key), lap)
    }

    pub fn personal_best_path(&self, key: &ReferenceLapKey) -> PathBuf {
        self.root.join(format!("{}.pb-lap", key.file_stem()))
    }

    fn legacy_personal_best_path(&self, key: &ReferenceLapKey) -> Option<PathBuf> {
        key.legacy_file_stem()
            .map(|file_stem| self.root.join(format!("{file_stem}.pb-lap")))
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

impl std::error::Error for StorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self { Self::Io(error) => Some(error), Self::InvalidFormat => None }
    }
}

fn validate_reference_lap(lap: &ReferenceLap) -> Result<(), StorageError> {
    if !lap.total_time_seconds.is_finite() || lap.total_time_seconds <= 0.0 {
        return Err(StorageError::InvalidFormat);
    }
    if lap.points.is_empty() || lap.points.len() > MAX_STORED_POINTS {
        return Err(StorageError::InvalidFormat);
    }
    if !lap.brake_threshold.is_finite() || !lap.throttle_threshold.is_finite()
        || !(0.0..=1.0).contains(&lap.brake_threshold) || !(0.0..=1.0).contains(&lap.throttle_threshold) {
        return Err(StorageError::InvalidFormat);
    }
    if lap.points.iter().any(|point| !point.is_valid()) {
        return Err(StorageError::InvalidFormat);
    }
    if lap.points.iter().any(|point| point.time_seconds > lap.total_time_seconds)
        || lap.points.windows(2).any(|pair| pair[0].progress >= pair[1].progress || pair[0].time_seconds > pair[1].time_seconds) {
        return Err(StorageError::InvalidFormat);
    }
    Ok(())
}

fn write_reference_lap(path: &Path, lap: &ReferenceLap) -> Result<(), StorageError> {
    let mut bytes = Vec::with_capacity(V2_HEADER_SIZE + lap.points.len() * POINT_SIZE);
    bytes.extend_from_slice(MAGIC_V2);
    bytes.extend_from_slice(&lap.total_time_seconds.to_le_bytes());
    bytes.extend_from_slice(&(lap.points.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&lap.brake_threshold.to_le_bytes());
    bytes.extend_from_slice(&lap.throttle_threshold.to_le_bytes());

    for point in &lap.points {
        bytes.extend_from_slice(&point.progress.to_le_bytes());
        bytes.extend_from_slice(&point.time_seconds.to_le_bytes());
        bytes.extend_from_slice(&point.throttle.to_le_bytes());
        bytes.extend_from_slice(&point.brake.to_le_bytes());
        bytes.extend_from_slice(&point.speed_kph.to_le_bytes());
        bytes.extend_from_slice(&point.gear.to_le_bytes());
        bytes.extend_from_slice(&point.steering.to_le_bytes());
    }

    atomic_write(path, &bytes)?;
    Ok(())
}

fn read_reference_lap(path: &Path) -> Result<ReferenceLap, StorageError> {
    let mut bytes = Vec::new();
    File::open(path)?.take((MAX_REFERENCE_BYTES + 1) as u64).read_to_end(&mut bytes)?;
    if bytes.len() < 20 || bytes.len() > MAX_REFERENCE_BYTES {
        return Err(StorageError::InvalidFormat);
    }
    let magic = bytes.get(..8).ok_or(StorageError::InvalidFormat)?;
    if !SUPPORTED_MAGIC
        .iter()
        .any(|supported| magic == supported.as_slice())
    {
        return Err(StorageError::InvalidFormat);
    }

    let total_time_seconds = read_f64(&bytes, 8)?;
    let point_count = read_u32(&bytes, 16)? as usize;
    if point_count == 0 || point_count > MAX_STORED_POINTS {
        return Err(StorageError::InvalidFormat);
    }
    let header_len = if magic == MAGIC_V2 { 36 } else { 20 };
    let expected_len = header_len + point_count * POINT_SIZE;
    if bytes.len() != expected_len {
        return Err(StorageError::InvalidFormat);
    }

    let (brake_threshold, throttle_threshold) = if magic == MAGIC_V2 {
        (read_f64(&bytes, 20)?, read_f64(&bytes, 28)?)
    } else {
        (0.10, 0.10)
    };
    let mut points = Vec::with_capacity(point_count);
    let mut offset = header_len;
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

    let lap = ReferenceLap { total_time_seconds, points, events: Vec::new(), brake_threshold, throttle_threshold };
    validate_reference_lap(&lap)?;
    ReferenceLap::from_points(lap.total_time_seconds, lap.points, lap.brake_threshold, lap.throttle_threshold)
    .ok_or(StorageError::InvalidFormat)
}

fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let temp_path = temp_path(path);
    let mut file = OpenOptions::new().write(true).create_new(true).open(&temp_path)?;
    let result = (|| {
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        replace_file(&temp_path, path)
    })();
    if result.is_err() { let _ = fs::remove_file(&temp_path); }
    result
}

fn temp_path(path: &Path) -> PathBuf {
    let mut temp_path = path.to_path_buf();
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| format!("{value}.tmp"))
        .unwrap_or_else(|| "tmp".to_string());
    temp_path.set_extension(format!(
        "{extension}.{}.{}",
        std::process::id(),
        TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    temp_path
}

#[cfg(windows)]
fn replace_file(temp_path: &Path, path: &Path) -> io::Result<()> {
    let temp = wide_path(temp_path);
    let target = wide_path(path);
    let replaced = unsafe {
        MoveFileExW(
            temp.as_ptr(),
            target.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if replaced == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(windows)]
fn wide_path(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain(Some(0)).collect()
}

#[cfg(not(windows))]
fn replace_file(temp_path: &Path, path: &Path) -> io::Result<()> {
    fs::rename(temp_path, path)
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

    struct TestStore {
        root: PathBuf,
        store: ReferenceLapStore,
    }

    impl TestStore {
        fn new(label: &str) -> Self {
            let root = env::temp_dir().join(format!(
                "hashoverlay-{label}-{}-{}",
                std::process::id(),
                TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            Self { store: ReferenceLapStore::new(&root), root }
        }

        fn reference() -> ReferenceLap {
            ReferenceLap::new(90.0, vec![point(0.0, 0.0), point(1.0, 90.0)]).unwrap()
        }
    }

    impl Drop for TestStore {
        fn drop(&mut self) { let _ = fs::remove_dir_all(&self.root); }
    }

    #[test]
    fn storage_errors_preserve_underlying_io_causes() {
        use std::error::Error;
        let error = StorageError::Io(io::Error::new(io::ErrorKind::PermissionDenied, "read denied"));
        assert_eq!(error.source().unwrap().to_string(), "read denied");
        assert!(StorageError::InvalidFormat.source().is_none());
    }

    #[test]
    fn corrupt_point_payloads_are_rejected_instead_of_silently_filtered() {
        let fixture = TestStore::new("corrupt-payload");
        let key = ReferenceLapKey::fallback();
        fixture.store.save_personal_best(&key, &TestStore::reference()).unwrap();
        let path = fixture.store.personal_best_path(&key);
        let original = fs::read(&path).unwrap();
        for offset in [0, 8, 16, 24, 32, 44] {
            let mut bytes = original.clone();
            let start = V2_HEADER_SIZE + POINT_SIZE + offset;
            bytes[start..start + 8].copy_from_slice(&f64::NAN.to_le_bytes());
            fs::write(&path, bytes).unwrap();
            assert!(matches!(fixture.store.load_personal_best(&key), Err(StorageError::InvalidFormat)));
        }
    }

    #[test]
    fn inconsistent_reference_chronology_is_rejected() {
        let fixture = TestStore::new("invalid-chronology");
        let mut lap = TestStore::reference();
        lap.points[100].time_seconds = 91.0;
        assert!(fixture.store.save_personal_best(&ReferenceLapKey::fallback(), &lap).is_err());
        lap = TestStore::reference();
        lap.points.swap(100, 101);
        assert!(fixture.store.save_personal_best(&ReferenceLapKey::fallback(), &lap).is_err());
        lap = TestStore::reference();
        lap.points[100].time_seconds = 0.0;
        assert!(fixture.store.save_personal_best(&ReferenceLapKey::fallback(), &lap).is_err());
    }

    #[test]
    fn out_of_range_reference_inputs_are_rejected() {
        let fixture = TestStore::new("out-of-range-point");
        for index in 0..6 {
            let mut lap = TestStore::reference();
            let point = &mut lap.points[1];
            match index { 0 => point.progress = 1.1, 1 => point.time_seconds = -1.0, 2 => point.throttle = 1.1, 3 => point.brake = -0.1, 4 => point.speed_kph = -1.0, _ => point.steering = 2.0 }
            assert!(matches!(fixture.store.save_personal_best(&ReferenceLapKey::fallback(), &lap), Err(StorageError::InvalidFormat)));
        }
    }

    #[test]
    fn nonfinite_point_values_cannot_overwrite_a_valid_reference() {
        let fixture = TestStore::new("nonfinite-point");
        let key = ReferenceLapKey::fallback();
        fixture.store.save_personal_best(&key, &TestStore::reference()).unwrap();
        for index in 0..6 {
            let mut lap = TestStore::reference();
            let point = &mut lap.points[1];
            match index { 0 => point.progress = f64::NAN, 1 => point.time_seconds = f64::NAN, 2 => point.throttle = f64::NAN, 3 => point.brake = f64::NAN, 4 => point.speed_kph = f64::NAN, _ => point.steering = f64::NAN }
            assert!(matches!(fixture.store.save_personal_best(&key, &lap), Err(StorageError::InvalidFormat)));
        }
        assert_eq!(fixture.store.load_personal_best(&key).unwrap().unwrap().total_time_seconds, 90.0);
    }

    #[test]
    fn invalid_driving_thresholds_are_not_persisted() {
        let fixture = TestStore::new("invalid-thresholds");
        for value in [f64::NAN, f64::INFINITY, -0.1, 1.1] {
            let mut lap = TestStore::reference();
            lap.brake_threshold = value;
            assert!(matches!(fixture.store.save_personal_best(&ReferenceLapKey::fallback(), &lap), Err(StorageError::InvalidFormat)));
            lap.brake_threshold = 0.1;
            lap.throttle_threshold = value;
            assert!(matches!(fixture.store.save_personal_best(&ReferenceLapKey::fallback(), &lap), Err(StorageError::InvalidFormat)));
        }
        assert!(!fixture.root.exists());
    }

    #[test]
    fn invalid_point_counts_do_not_create_reference_directories() {
        let fixture = TestStore::new("invalid-points");
        let mut lap = TestStore::reference();
        lap.points.clear();
        assert!(matches!(fixture.store.save_personal_best(&ReferenceLapKey::fallback(), &lap), Err(StorageError::InvalidFormat)));
        lap.points = vec![point(0.0, 0.0); MAX_STORED_POINTS + 1];
        assert!(matches!(fixture.store.save_personal_best(&ReferenceLapKey::fallback(), &lap), Err(StorageError::InvalidFormat)));
        assert!(!fixture.root.exists());
    }

    #[test]
    fn failed_reference_replacement_removes_its_temporary_file() {
        let fixture = TestStore::new("failed-replacement");
        let key = ReferenceLapKey::fallback();
        fs::create_dir_all(fixture.store.personal_best_path(&key)).unwrap();
        assert!(fixture.store.save_personal_best(&key, &TestStore::reference()).is_err());
        assert_eq!(fs::read_dir(&fixture.root).unwrap().count(), 1);
    }

    #[test]
    fn oversized_reference_files_are_rejected() {
        let fixture = TestStore::new("oversized-reference");
        fs::create_dir_all(&fixture.root).unwrap();
        let path = fixture.store.personal_best_path(&ReferenceLapKey::fallback());
        File::create(path).unwrap().set_len(2_000_000).unwrap();
        assert!(matches!(fixture.store.load_personal_best(&ReferenceLapKey::fallback()), Err(StorageError::InvalidFormat)));
    }

    #[test]
    fn invalid_duration_never_replaces_existing_reference() {
        let fixture = TestStore::new("invalid-write");
        let key = ReferenceLapKey::fallback();
        let valid = TestStore::reference();
        fixture.store.save_personal_best(&key, &valid).unwrap();
        let mut invalid = valid.clone();
        invalid.total_time_seconds = f64::NAN;
        assert!(matches!(fixture.store.save_personal_best(&key, &invalid), Err(StorageError::InvalidFormat)));
        assert_eq!(fixture.store.load_personal_best(&key).unwrap().unwrap().total_time_seconds, 90.0);
    }

    #[test]
    fn missing_reference_does_not_create_a_storage_directory() {
        let fixture = TestStore::new("missing-reference");
        assert!(fixture.store.load_personal_best(&ReferenceLapKey::fallback()).unwrap().is_none());
        assert!(!fixture.root.exists());
    }

    #[test]
    fn saves_and_loads_reference_lap() {
        let root = env::temp_dir().join(format!("hashoverlay-storage-test-{}", std::process::id()));
        let store = ReferenceLapStore::new(&root);
        let key = ReferenceLapKey::fallback();
        let lap = ReferenceLap::from_points(
            90.0,
            vec![point(0.0, 0.0), point(0.5, 45.0), point(1.0, 90.0)],
            0.22,
            0.31,
        )
        .unwrap();

        store.save_personal_best(&key, &lap).unwrap();
        let loaded = store.load_personal_best(&key).unwrap().unwrap();

        assert_eq!(loaded.total_time_seconds, 90.0);
        assert_eq!(loaded.points.len(), 2_001);
        assert_eq!(loaded.brake_threshold, 0.22);
        assert_eq!(loaded.throttle_threshold, 0.31);
    }

    #[test]
    fn sanitizes_lap_keys_for_paths() {
        let key = ReferenceLapKey {
            track: "Le Mans/24h".to_string(),
            track_layout: "2026".to_string(),
            car: "Car:Hyper".to_string(),
            legacy_vehicle_class: Some("Hypercar".to_string()),
        };

        assert_eq!(key.file_stem(), "le-mans-24h__2026__car-hyper");
        assert_eq!(
            key.legacy_file_stem().as_deref(),
            Some("le-mans-24h__car-hyper__hypercar")
        );

        let no_layout = ReferenceLapKey {
            track: "Le Mans/24h".to_string(),
            track_layout: String::new(),
            car: "Car:Hyper".to_string(),
            legacy_vehicle_class: None,
        };
        assert_eq!(no_layout.file_stem(), "le-mans-24h__car-hyper");
    }

    #[test]
    fn loads_legacy_vehicle_class_personal_best_path() {
        let root = env::temp_dir().join(format!(
            "hashoverlay-storage-legacy-key-{}",
            std::process::id()
        ));
        let store = ReferenceLapStore::new(&root);
        fs::create_dir_all(&root).unwrap();
        let key = ReferenceLapKey {
            track: "Sebring".to_string(),
            track_layout: "unknown-layout".to_string(),
            car: "Porsche 963".to_string(),
            legacy_vehicle_class: Some("Hypercar".to_string()),
        };
        let legacy_path = root.join("sebring__porsche-963__hypercar.pb-lap");
        let lap = ReferenceLap::new(
            90.0,
            vec![point(0.0, 0.0), point(0.5, 45.0), point(1.0, 90.0)],
        )
        .unwrap();

        write_reference_lap(&legacy_path, &lap).unwrap();

        let loaded = store.load_personal_best(&key).unwrap().unwrap();
        assert_eq!(loaded.total_time_seconds, 90.0);
    }

    #[test]
    fn rejects_invalid_magic() {
        let root = env::temp_dir().join(format!(
            "hashoverlay-storage-invalid-magic-{}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        let path = root.join("bad.pb-lap");
        fs::write(&path, b"NOTALAP1").unwrap();

        assert!(matches!(
            read_reference_lap(&path),
            Err(StorageError::InvalidFormat)
        ));
    }

    #[test]
    fn rejects_absurd_point_count() {
        let root = env::temp_dir().join(format!(
            "hashoverlay-storage-absurd-count-{}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        let path = root.join("bad.pb-lap");
        let mut bytes = Vec::new();
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(&90.0f64.to_le_bytes());
        bytes.extend_from_slice(&u32::MAX.to_le_bytes());
        fs::write(&path, bytes).unwrap();

        assert!(matches!(
            read_reference_lap(&path),
            Err(StorageError::InvalidFormat)
        ));
    }

    #[test]
    fn ignores_interrupted_temp_file() {
        let root = env::temp_dir().join(format!(
            "hashoverlay-storage-temp-file-{}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        let store = ReferenceLapStore::new(&root);
        let key = ReferenceLapKey::fallback();
        fs::write(temp_path(&store.personal_best_path(&key)), b"partial").unwrap();

        assert!(store.load_personal_best(&key).unwrap().is_none());
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

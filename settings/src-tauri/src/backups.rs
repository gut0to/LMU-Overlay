use std::{fs, io::{self, Read}, path::{Path, PathBuf}};
use std::{io::Write, sync::atomic::{AtomicU64, Ordering}, time::{SystemTime, UNIX_EPOCH}};
use serde::Serialize;

pub const MAX_BACKUP_BYTES: u64 = 4 * 1024 * 1024;
pub const BACKUP_LIMIT: usize = 5;
static NEXT_BACKUP: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Serialize)]
pub struct BackupInfo { pub id: String, pub modified_ms: u64, pub size_bytes: u64 }

pub fn create(config: &Path) -> Result<BackupInfo, String> {
    let text = read_bounded(config).map_err(|error| format!("Could not back up the saved configuration: {error}"))?;
    let root = folder(config);
    fs::create_dir_all(&root).map_err(|error| error.to_string())?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|error| error.to_string())?;
    let id = format!("{:039}-{:010}-{:020}.toml", now.as_nanos(), std::process::id(), NEXT_BACKUP.fetch_add(1, Ordering::Relaxed));
    let target = root.join(&id);
    let temporary = target.with_extension("pending");
    let mut file = fs::OpenOptions::new().write(true).create_new(true).open(&temporary).map_err(|error| error.to_string())?;
    let written = file.write_all(text.as_bytes()).and_then(|_| file.sync_all());
    drop(file);
    let result = written.and_then(|_| fs::rename(&temporary, &target));
    if let Err(error) = result { let _ = fs::remove_file(&temporary); return Err(format!("Could not publish configuration backup: {error}")); }
    Ok(BackupInfo { id, modified_ms: now.as_millis().min(u128::from(u64::MAX)) as u64, size_bytes: text.len() as u64 })
}

fn folder(config: &Path) -> PathBuf { config.with_extension("backups") }

pub fn list(config: &Path) -> Result<Vec<BackupInfo>, String> {
    let entries = match fs::read_dir(folder(config)) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("Could not list configuration backups: {error}")),
    };
    let mut backups = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        let id = entry.file_name().to_string_lossy().into_owned();
        if !valid_id(&id) || !entry.file_type().map_err(|error| error.to_string())?.is_file() { continue; }
        let metadata = entry.metadata().map_err(|error| error.to_string())?;
        let modified_ms = metadata.modified().ok().and_then(|time| time.duration_since(UNIX_EPOCH).ok()).map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64).unwrap_or(0);
        backups.push(BackupInfo { id, modified_ms, size_bytes: metadata.len() });
    }
    backups.sort_by(|left, right| right.id.cmp(&left.id));
    Ok(backups)
}

fn valid_id(id: &str) -> bool {
    let Some(stem) = id.strip_suffix(".toml") else { return false; };
    !stem.is_empty() && stem.len() <= 96 && stem.as_bytes()[0].is_ascii_digit()
        && stem.bytes().all(|value| value.is_ascii_digit() || value == b'-')
}

pub fn read(config: &Path, id: &str) -> Result<String, String> {
    if !valid_id(id) { return Err("Invalid configuration backup identifier".into()); }
    read_bounded(&folder(config).join(id)).map_err(|error| format!("Could not read configuration backup: {error}"))
}

fn read_bounded(path: &Path) -> io::Result<String> {
    let mut bytes = Vec::new();
    fs::File::open(path)?.take(MAX_BACKUP_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_BACKUP_BYTES { return Err(io::Error::new(io::ErrorKind::InvalidData, "configuration exceeds the 4 MiB backup limit")); }
    String::from_utf8(bytes).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

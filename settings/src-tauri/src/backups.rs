use std::{fs, io::{self, Read}, path::{Path, PathBuf}};

pub const MAX_BACKUP_BYTES: u64 = 4 * 1024 * 1024;
pub const BACKUP_LIMIT: usize = 5;

fn folder(config: &Path) -> PathBuf { config.with_extension("backups") }

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

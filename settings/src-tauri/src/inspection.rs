use std::{fs::File, io::Read, path::Path};
use overlay_renderer::config::OverlayConfig;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct SavedFile { pub path: String, pub revision: String, pub config: OverlayConfig, pub text: String }

pub fn read(path: &Path) -> Result<SavedFile, String> {
    let mut bytes = Vec::new();
    File::open(path).map_err(|error| error.to_string())?.take(4 * 1024 * 1024 + 1).read_to_end(&mut bytes).map_err(|error| error.to_string())?;
    if bytes.len() > 4 * 1024 * 1024 { return Err("Saved configuration exceeds the 4 MiB inspection limit".into()); }
    let text = String::from_utf8(bytes).map_err(|error| error.to_string())?;
    let revision = OverlayConfig::text_revision(&text).to_string();
    let config = OverlayConfig::from_import_text(&text).map_err(|error| error.to_string())?;
    Ok(SavedFile { path: path.display().to_string(), revision, config, text })
}

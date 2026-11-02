use overlay_renderer::config::OverlayConfig;
use serde::Serialize;
use std::{fs::File, io::Read, path::Path};

#[derive(Debug, Serialize)]
pub struct SavedFile {
    pub path: String,
    pub revision: String,
    pub config: OverlayConfig,
    pub text: String,
}

pub fn read(path: &Path) -> Result<SavedFile, String> {
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|error| error.to_string())?
        .take(4 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() > 4 * 1024 * 1024 {
        return Err("Saved configuration exceeds the 4 MiB inspection limit".into());
    }
    let text = String::from_utf8(bytes).map_err(|error| error.to_string())?;
    let revision = OverlayConfig::text_revision(&text).to_string();
    let config = OverlayConfig::from_import_text(&text).map_err(|error| error.to_string())?;
    Ok(SavedFile {
        path: path.display().to_string(),
        revision,
        config,
        text,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    #[test]
    fn formatting_only_edits_change_the_revision_even_when_normalized_values_match() {
        let path = std::env::temp_dir().join(format!(
            "hashoverlay-inspect-comments-{}.toml",
            std::process::id()
        ));
        fs::write(&path, "config_version = 7\n").unwrap();
        let before = read(&path).unwrap();
        fs::write(&path, "# external comment\nconfig_version = 7\n").unwrap();
        let after = read(&path).unwrap();
        assert_eq!(before.config, after.config);
        assert_ne!(before.revision, after.revision);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn inspection_migrates_in_memory_without_rewriting_legacy_files_or_creating_backups() {
        let path = std::env::temp_dir().join(format!(
            "hashoverlay-inspect-legacy-{}.toml",
            std::process::id()
        ));
        let original = "# user comment\nconfig_version = 3\n[window]\nx = -1920\n";
        fs::write(&path, original).unwrap();
        let snapshot = read(&path).unwrap();
        assert_eq!(snapshot.text, original);
        assert_eq!(
            snapshot.revision,
            OverlayConfig::revision(&path).unwrap().to_string()
        );
        assert_eq!(snapshot.config.window.x, -1920);
        assert!(snapshot.config.config_version > 3);
        assert_eq!(fs::read_to_string(&path).unwrap(), original);
        assert!(!path.with_extension("toml.v3.bak").exists());
        fs::remove_file(path).unwrap();
    }
    #[test]
    fn malformed_future_and_oversized_saved_files_are_reported_without_replacement() {
        let path = std::env::temp_dir().join(format!(
            "hashoverlay-inspect-invalid-{}.toml",
            std::process::id()
        ));
        for text in [
            "invalid TOML".to_string(),
            "config_version = 99999".into(),
            "#".repeat(4 * 1024 * 1024 + 1),
        ] {
            fs::write(&path, &text).unwrap();
            assert!(read(&path).is_err());
            assert_eq!(fs::read_to_string(&path).unwrap(), text);
        }
        fs::remove_file(path).unwrap();
    }
}

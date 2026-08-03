//! settings.json read/write with corruption recovery and atomic writes.
//!
//! Config file location: `%USERPROFILE%\.oasis-companion\settings.json`
//! (Agent-neutral — not tied to any specific Agent's directory).

pub mod schema;

pub use schema::*;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Result of loading settings: the parsed value plus a flag indicating the
/// on-disk file was corrupt/missing and had to be (re)created from defaults.
pub struct LoadResult {
    pub settings: Settings,
    pub created_default: bool,
    pub recovered: bool,
}

/// Absolute path to the runtime settings file.
pub fn settings_path() -> PathBuf {
    settings_dir().join("settings.json")
}

/// Absolute path to the runtime settings directory.
pub fn settings_dir() -> PathBuf {
    home_dir().join(".oasis-companion")
}

/// Resolve `%USERPROFILE%` to an absolute path.
pub fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

/// Expand a leading `%USERPROFILE%` token in a stored path to an absolute path.
pub fn expand_user_profile(p: &str) -> PathBuf {
    let lower = p.to_ascii_lowercase();
    if lower.strip_prefix("%userprofile%").is_some() {
        // Reconstruct using the original casing of the suffix.
        let suffix = &p["%USERPROFILE%".len()..];
        home_dir().join(suffix.trim_start_matches(['\\', '/']))
    } else {
        PathBuf::from(p)
    }
}

/// Load settings. If the file is missing or unparseable, it is (re)created from
/// defaults; a corrupt file is backed up as `settings.corrupt.<ts>.json` before
/// being replaced. Never returns an error for a missing/corrupt file — only for
/// truly fatal IO failures.
pub fn load() -> Result<LoadResult, Box<dyn std::error::Error>> {
    let path = settings_path();
    if !path.exists() {
        log::info!("settings.json not found, writing defaults");
        let s = Settings::default();
        save(&s)?;
        return Ok(LoadResult {
            settings: s,
            created_default: true,
            recovered: false,
        });
    }

    let raw = fs::read_to_string(&path);
    let raw = match raw {
        Ok(s) => s,
        Err(e) => {
            log::warn!("settings.json unreadable ({}), backing up + resetting", e);
            backup_and_reset(&path)?;
            let s = Settings::default();
            save(&s)?;
            return Ok(LoadResult {
                settings: s,
                created_default: false,
                recovered: true,
            });
        }
    };

    match serde_json::from_str::<Settings>(&raw) {
        Ok(s) => Ok(LoadResult {
            settings: s,
            created_default: false,
            recovered: false,
        }),
        Err(e) => {
            log::warn!("settings.json parse error ({}), backing up + resetting", e);
            backup_and_reset(&path)?;
            let s = Settings::default();
            save(&s)?;
            Ok(LoadResult {
                settings: s,
                created_default: false,
                recovered: true,
            })
        }
    }
}

/// Atomic write: serialize to a temp file in the same dir, then rename over the
/// target. Avoids half-written files if the process is interrupted.
pub fn save(settings: &Settings) -> Result<(), Box<dyn std::error::Error>> {
    save_to_path(settings, &settings_path())
}

fn save_to_path(settings: &Settings, target: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let dir = target
        .parent()
        .ok_or_else(|| format!("settings path has no parent: {:?}", target))?;
    fs::create_dir_all(dir)?;

    let json = serde_json::to_string_pretty(settings)?;
    let tmp = dir.join(".settings.json.tmp");

    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(json.as_bytes())?;
        f.sync_all()?;
    }
    if target.exists() {
        fs::remove_file(target)?;
    }
    fs::rename(&tmp, target)?;
    Ok(())
}

fn backup_and_reset(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let backup = path.with_extension(format!("corrupt.{}.json", ts));
    if path.exists() {
        let _ = fs::rename(path, &backup);
        log::info!("backed up corrupt settings to {:?}", backup);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_can_replace_an_existing_settings_file() {
        let dir = std::env::temp_dir().join(format!(
            "oasis-companion-config-test-{}",
            std::process::id()
        ));
        let path = dir.join("settings.json");
        let _ = fs::remove_dir_all(&dir);

        let settings = Settings::default();
        save_to_path(&settings, &path).expect("first save should create settings.json");

        let mut next = settings.clone();
        next.agent_detection.interval_seconds = 7;
        save_to_path(&next, &path).expect("second save should replace settings.json");

        let raw = fs::read_to_string(&path).expect("settings.json should be readable");
        let parsed: Settings =
            serde_json::from_str(&raw).expect("settings.json should remain valid");
        assert_eq!(parsed.agent_detection.interval_seconds, 7);

        let _ = fs::remove_dir_all(&dir);
    }
}

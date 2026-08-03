//! Windows autostart via `tauri-plugin-autostart` (HKCU Run key, per-user, no UAC).
//!
//! The plugin is registered in `lib::run`. These helpers toggle/check it.

use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;

pub fn enable(app: &AppHandle) -> Result<(), String> {
    app.autolaunch().enable().map_err(|e| e.to_string())
}

pub fn disable(app: &AppHandle) -> Result<(), String> {
    app.autolaunch().disable().map_err(|e| e.to_string())
}

pub fn is_enabled(app: &AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

/// Apply the autostart setting from config on startup.
pub fn sync_from_config(app: &AppHandle, want_enabled: bool) {
    let current = is_enabled(app).unwrap_or(false);
    if want_enabled && !current {
        let _ = enable(app);
    } else if !want_enabled && current {
        let _ = disable(app);
    }
}

// Re-exported so lib.rs can call `autostart::plugin()`.
use tauri_plugin_autostart::MacosLauncher;
pub fn plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--background"]))
}

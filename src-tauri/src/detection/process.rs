//! Process-based Agent detection via `sysinfo`.
//!
//! Matches by executable name (primary) and full exe path (most precise, used
//! to disambiguate same-named processes). Refreshes the full process list each
//! poll; for a 3s interval the cost is negligible.

use sysinfo::System;

/// True if any running process matches the configured names or install paths.
///
/// Matching is case-insensitive. `process_names` are bare exe names (e.g.
/// `Codex.exe`); `install_paths` are absolute exe paths.
pub fn any_agent_running(process_names: &[String], install_paths: &[String]) -> bool {
    if process_names.is_empty() && install_paths.is_empty() {
        return false;
    }

    let names: Vec<String> = process_names
        .iter()
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .collect();
    let paths: Vec<String> = install_paths
        .iter()
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .collect();

    let sys = System::new_all();
    for proc in sys.processes().values() {
        let pname = proc.name().to_ascii_lowercase();
        if names.iter().any(|n| n == &pname) {
            return true;
        }
        if let Some(exe) = proc.exe() {
            let pexe = exe.to_string_lossy().to_ascii_lowercase();
            if paths.contains(&pexe) {
                return true;
            }
        }
    }
    false
}

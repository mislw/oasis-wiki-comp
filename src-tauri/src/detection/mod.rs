//! Agent-detection polling loop.
//!
//! Spawns a background task that, every `interval_seconds`, checks whether a
//! configured Agent process is running and recomputes the ball state. All
//! `AppState` locks are held briefly and never across the `sleep` await.

pub mod process;
pub mod window;

use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};

use crate::state::AppState;

/// Spawn the detection loop. Runs for the lifetime of the app.
pub fn spawn_loop(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        log::info!("detection loop started");
        loop {
            // --- read config (brief lock, dropped before await) ---
            let (interval, enabled, paused, follow_lifecycle, names, paths, titles) = {
                let st = app.state::<AppState>();
                let s = st.settings.lock().unwrap();
                (
                    s.agent_detection.interval_seconds,
                    s.agent_detection.enabled,
                    s.companion.pause_detection,
                    s.companion.follow_agent_lifecycle,
                    s.agent_detection.process_names.clone(),
                    s.agent_detection.install_paths.clone(),
                    s.agent_detection.window_titles.clone(),
                )
            };

            // --- detect (no locks held) ---
            let detected_targets =
                crate::detection::process::active_target_ids(&crate::agent_registry::all_targets());
            let present = if !enabled || paused {
                false
            } else {
                process::any_agent_running(&names, &paths)
                    || window::any_window_matches(&titles)
                    || !detected_targets.is_empty()
            };

            let st = app.state::<AppState>();
            let prev_present = st
                .agent_present
                .swap(present, std::sync::atomic::Ordering::Relaxed);
            let prev_targets = {
                let mut lock = st.active_targets.lock().unwrap();
                let prev = lock.clone();
                *lock = detected_targets.clone();
                prev
            };
            if prev_present != present {
                log::info!("agent presence changed: {} -> {}", prev_present, present);
                let _ = app.emit("agent://presence", present);
            }
            if prev_targets != detected_targets {
                log::info!(
                    "active agent targets changed: {:?} -> {:?}",
                    prev_targets,
                    detected_targets
                );
                let _ = app.emit("agent://active-targets", detected_targets.clone());
            }
            if follow_lifecycle && !present {
                crate::tray::hide_settings(&app);
            }

            // --- recompute + apply ball state (brief lock) ---
            let (new_state, changed) = {
                let st = app.state::<AppState>();
                let ns = st.compute_state();
                let mut bs = st.ball_state.lock().unwrap();
                let changed = *bs != ns;
                *bs = ns;
                (ns, changed)
            };
            if changed {
                crate::ball::apply_state(&app, new_state);
            }

            tokio::time::sleep(Duration::from_secs(interval.max(1))).await;
        }
    });
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettingsVisibilityAction {
    Keep,
    Hide,
}

fn settings_visibility_action(
    follow_lifecycle: bool,
    agent_present: bool,
) -> SettingsVisibilityAction {
    if follow_lifecycle && !agent_present {
        SettingsVisibilityAction::Hide
    } else {
        SettingsVisibilityAction::Keep
    }
}

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
            if settings_visibility_action(follow_lifecycle, present)
                == SettingsVisibilityAction::Hide
            {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detecting_an_agent_never_requests_a_settings_popup() {
        assert_eq!(
            settings_visibility_action(true, true),
            SettingsVisibilityAction::Keep
        );
        assert_eq!(
            settings_visibility_action(false, true),
            SettingsVisibilityAction::Keep
        );
    }

    #[test]
    fn lifecycle_following_hides_settings_after_the_agent_exits() {
        assert_eq!(
            settings_visibility_action(true, false),
            SettingsVisibilityAction::Hide
        );
        assert_eq!(
            settings_visibility_action(false, false),
            SettingsVisibilityAction::Keep
        );
    }
}

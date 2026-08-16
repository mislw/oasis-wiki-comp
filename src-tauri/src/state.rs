//! Shared application state and the floating-ball state machine.

use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use crate::config::Settings;
use crate::skill::MultiTargetStatus;
use crate::ui_workbench_catalog::WorkbenchCatalog;
use crate::ui_workflow::UiWorkflowStore;

/// The four visual states of the floating ball.
///
/// `Error` and `UpdateAvailable` are *overlay* states — they only show while an
/// Agent is running (`Idle` base). MVP never produces `UpdateAvailable`.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BallState {
    /// No Agent detected — ball not shown.
    Hidden,
    /// Agent running, everything healthy.
    Idle,
    /// Agent running but Skill not installed / config corrupt.
    Error,
}

impl BallState {
    pub fn label(&self) -> &'static str {
        match self {
            BallState::Hidden => "hidden",
            BallState::Idle => "idle",
            BallState::Error => "error",
        }
    }
}

/// Shared application state shared between the detection loop, tray, and IPC commands.
pub struct AppState {
    pub settings: Mutex<Settings>,
    pub ball_state: Mutex<BallState>,
    pub agent_present: AtomicBool,
    pub active_targets: Mutex<Vec<String>>,
    pub update_available: AtomicBool,
    pub skill_status: Mutex<MultiTargetStatus>,
    /// Latest validated external session URL for the UI Workbench window.
    pub pending_ui_workbench_url: Mutex<Option<String>>,
    /// Generated UI pages available across Companion restarts.
    pub ui_workbench_catalog: Mutex<WorkbenchCatalog>,
    /// Persistent eight-stage state for each generated UI page.
    pub ui_workflow_store: Mutex<UiWorkflowStore>,
    /// True if settings.json had to be recovered from corruption this session.
    pub config_error: AtomicBool,
}

impl AppState {
    #[cfg(test)]
    pub fn new(settings: Settings, skill_status: MultiTargetStatus, config_error: bool) -> Self {
        Self::with_catalog(
            settings,
            skill_status,
            config_error,
            WorkbenchCatalog::default(),
            UiWorkflowStore::default(),
        )
    }

    pub fn with_catalog(
        settings: Settings,
        skill_status: MultiTargetStatus,
        config_error: bool,
        ui_workbench_catalog: WorkbenchCatalog,
        ui_workflow_store: UiWorkflowStore,
    ) -> Self {
        AppState {
            settings: Mutex::new(settings),
            ball_state: Mutex::new(BallState::Hidden),
            agent_present: AtomicBool::new(false),
            active_targets: Mutex::new(Vec::new()),
            update_available: AtomicBool::new(false),
            skill_status: Mutex::new(skill_status),
            pending_ui_workbench_url: Mutex::new(None),
            ui_workbench_catalog: Mutex::new(ui_workbench_catalog),
            ui_workflow_store: Mutex::new(ui_workflow_store),
            config_error: AtomicBool::new(config_error),
        }
    }

    /// Compute the ball state from the current flags.
    pub fn compute_state(&self) -> BallState {
        let show_on_agent = self.settings.lock().unwrap().ball.show_on_agent;
        if !show_on_agent {
            return BallState::Hidden;
        }
        if !self.agent_present.load(Ordering::Relaxed) {
            return BallState::Hidden;
        }
        if self.config_error.load(Ordering::Relaxed) {
            return BallState::Error;
        }
        BallState::Idle
    }
}

#[cfg(test)]
fn installed_status() -> MultiTargetStatus {
    MultiTargetStatus {
        targets: vec![crate::skill::TargetStatus {
            target_id: "codex".into(),
            display_name: "Codex".into(),
            status: crate::skill::SkillStatus::Installed,
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    #[test]
    fn hides_ball_when_show_on_agent_is_disabled_even_if_agent_is_present() {
        let mut settings = Settings::default();
        settings.ball.show_on_agent = false;
        let state = AppState::new(settings, installed_status(), false);
        state.agent_present.store(true, Ordering::Relaxed);

        assert_eq!(state.compute_state(), BallState::Hidden);
    }

    #[test]
    fn initial_default_config_is_not_an_error_state() {
        let state = AppState::new(Settings::default(), installed_status(), false);
        state.agent_present.store(true, Ordering::Relaxed);

        assert_eq!(state.compute_state(), BallState::Idle);
    }

    #[test]
    fn skill_health_does_not_turn_ball_error() {
        let status = MultiTargetStatus {
            targets: vec![crate::skill::TargetStatus {
                target_id: "claude-code".into(),
                display_name: "Claude Code".into(),
                status: crate::skill::SkillStatus::NotInstalled,
            }],
        };
        let state = AppState::new(Settings::default(), status, false);
        state.agent_present.store(true, Ordering::Relaxed);

        assert_eq!(state.compute_state(), BallState::Idle);
    }

    #[test]
    fn update_available_does_not_change_ball_state() {
        let state = AppState::new(Settings::default(), installed_status(), false);
        state.agent_present.store(true, Ordering::Relaxed);
        state.update_available.store(true, Ordering::Relaxed);

        assert_eq!(state.compute_state(), BallState::Idle);
    }
}

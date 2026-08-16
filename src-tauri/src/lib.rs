//! Oasis Companion - Tauri 2 backend assembly.
//!
//! Wires plugins, shared state, the detection loop, tray, and IPC commands.

mod agent_registry;
mod autostart;
mod ball;
mod codex_delivery;
mod commands;
mod companion_handoff;
mod config;
mod detection;
mod mcp;
mod skill;
mod state;
mod tray;
mod ui_delivery_preflight;
mod ui_workbench;
mod ui_workbench_catalog;
mod ui_workflow;
mod updater;

use std::sync::atomic::Ordering;
use tauri::{AppHandle, Manager};

/// Re-detect Skill status, update shared state, and recompute the ball.
pub fn refresh_skill_status(app: &AppHandle) {
    let targets = app
        .state::<state::AppState>()
        .settings
        .lock()
        .unwrap()
        .skill
        .targets
        .clone();
    let status = skill::detect_status(&targets);
    {
        let st = app.state::<state::AppState>();
        *st.skill_status.lock().unwrap() = status.clone();
    }
    log::info!("skill status: {:?}", status);

    let ns = app.state::<state::AppState>().compute_state();
    let changed = {
        let st = app.state::<state::AppState>();
        let mut bs = st.ball_state.lock().unwrap();
        let c = *bs != ns;
        *bs = ns;
        c
    };
    if changed {
        ball::apply_state(app, ns);
    }
}

pub async fn refresh_update_status(app: AppHandle) -> updater::UpdateStatus {
    updater::check(&app).await
}

pub async fn refresh_mcp_status(app: AppHandle) -> mcp::McpStatus {
    mcp::check(&app).await
}

/// Launch the configured Agent executable (used by the "Oasis" shortcut
/// and the single-instance `--launch-agent` arg).
pub fn launch_agent(app: &AppHandle) {
    let path = app
        .state::<state::AppState>()
        .settings
        .lock()
        .unwrap()
        .companion
        .agent_launch_path
        .clone();
    match path {
        Some(p) if std::path::Path::new(&p).exists() => {
            match std::process::Command::new(&p).spawn() {
                Ok(_) => log::info!("launched agent: {}", p),
                Err(e) => {
                    log::error!("launch agent failed: {}", e);
                    tray::show_settings(app);
                }
            }
        }
        _ => {
            log::warn!("agent path not configured, opening settings");
            tray::show_settings(app);
        }
    }
}

/// Toggle Agent detection: persist `pause_detection` and hide the ball if pausing.
pub fn apply_pause(app: &AppHandle, paused: bool) -> Result<(), String> {
    let mut s = app
        .state::<state::AppState>()
        .settings
        .lock()
        .unwrap()
        .clone();
    s.companion.pause_detection = paused;
    config::save(&s).map_err(|e| e.to_string())?;
    *app.state::<state::AppState>().settings.lock().unwrap() = s;
    log::info!("agent detection paused={}", paused);
    if paused {
        app.state::<state::AppState>()
            .agent_present
            .store(false, Ordering::Relaxed);
        app.state::<state::AppState>()
            .active_targets
            .lock()
            .unwrap()
            .clear();
        *app.state::<state::AppState>().ball_state.lock().unwrap() = state::BallState::Hidden;
        ball::apply_state(app, state::BallState::Hidden);
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Windows are created before `setup()`, and their frontend may invoke IPC
    // immediately. Register shared state on the builder before window creation.
    let load = config::load().unwrap_or_else(|e| {
        eprintln!("config load failed: {e} - using defaults");
        config::LoadResult {
            settings: config::Settings::default(),
            created_default: false,
            recovered: true,
        }
    });
    let created_default = load.created_default;
    let autostart_want = load.settings.companion.autostart;
    let skill_status = skill::detect_status(&load.settings.skill.targets);
    let initial_state = state::AppState::with_catalog(
        load.settings,
        skill_status.clone(),
        load.recovered,
        ui_workbench_catalog::load_catalog(),
        ui_workflow::load_store(),
    );
    let initial_workbench_url = ui_workbench::handoff_action(&std::env::args().collect::<Vec<_>>());
    let initial_workflow_update =
        ui_workflow::parse_update_arg(&std::env::args().collect::<Vec<_>>());
    let initial_workflow_open =
        ui_workflow::is_open_requested(&std::env::args().collect::<Vec<_>>());

    // Single-instance: a second launch forwards its args to this callback (runs
    // in the already-running instance), then the second process exits.
    let single_instance = tauri_plugin_single_instance::init(|app, argv, _cwd| {
        log::info!("second instance args: {:?}", argv);
        match ui_workflow::parse_update_arg(&argv) {
            Ok(Some(path)) => {
                if let Err(error) = ui_workflow::apply_update_handoff(app, &path) {
                    log::error!("UI workflow update failed: {error}");
                }
                return;
            }
            Ok(None) => {}
            Err(error) => {
                log::warn!("rejected UI workflow update: {error}");
                return;
            }
        }
        match ui_workbench::handoff_action(&argv) {
            Ok(Some(handoff)) => {
                if let Err(error) = ui_workbench::apply_ui_workbench_handoff(app, handoff) {
                    log::error!("UI Workbench handoff failed: {error}");
                }
            }
            Ok(None) => {
                if ui_workflow::is_open_requested(&argv) {
                    if let Err(error) = ui_workflow::show_ui_workflow(app) {
                        log::error!("UI workflow window failed: {error}");
                    }
                } else if argv.iter().any(|a| a == "--launch-agent") {
                    launch_agent(app);
                } else if argv.iter().any(|a| a == "--background") {
                    log::info!("second background launch ignored");
                } else {
                    tray::show_settings(app);
                }
            }
            Err(error) => log::warn!("rejected UI Workbench handoff: {error}"),
        }
    });

    tauri::Builder::default()
        .manage(initial_state)
        // single-instance MUST be registered first.
        .plugin(single_instance)
        .plugin(autostart::plugin())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_log::Builder::new().build())
        .on_window_event(tray::handle_window_event)
        .setup(move |app| {
            if created_default {
                log::info!("created default settings.json");
            }
            log::info!("initial skill status: {:?}", skill_status);

            // --- tray ---
            tray::build(app.handle())?;

            let inbox_handoff =
                companion_handoff::initialize(app.handle()).map_err(std::io::Error::other)?;

            // --- detection loop ---
            detection::spawn_loop(app.handle().clone());
            tray::spawn_settings_popover_follow_loop(app.handle().clone());

            if app
                .state::<state::AppState>()
                .settings
                .lock()
                .unwrap()
                .updates
                .auto_check
            {
                let app_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    let status = refresh_update_status(app_handle).await;
                    log::info!("update check: {:?}", status);
                });
            }

            // --- startup window policy ---
            // `--background` (autostart) => stay in tray only.
            // direct launch => show settings so the user gets feedback.
            let mut opened_workbench = inbox_handoff;
            match &initial_workflow_update {
                Ok(Some(path)) => {
                    ui_workflow::apply_update_handoff(app.handle(), path)
                        .map_err(std::io::Error::other)?;
                    opened_workbench = true;
                }
                Ok(None) => {}
                Err(error) => log::warn!("rejected UI workflow update: {error}"),
            }
            match &initial_workbench_url {
                Ok(Some(handoff)) => {
                    ui_workbench::apply_ui_workbench_handoff(app.handle(), handoff.clone())
                        .map_err(std::io::Error::other)?;
                    opened_workbench = true;
                }
                Ok(None) => {}
                Err(error) => log::warn!("rejected UI Workbench handoff: {error}"),
            }
            if initial_workflow_open {
                ui_workflow::show_ui_workflow(app.handle()).map_err(std::io::Error::other)?;
                opened_workbench = true;
            }
            let background = std::env::args().any(|a| a == "--background");
            if !background && !opened_workbench {
                tray::show_settings(app.handle());
            }

            // --- sync autostart to config ---
            let skip_autostart_sync = std::env::args().any(|a| a == "--no-autostart-sync");
            if skip_autostart_sync {
                log::info!("skipping autostart sync because --no-autostart-sync was provided");
            } else {
                let app_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    autostart::sync_from_config(&app_handle, autostart_want);
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::get_ball_state,
            commands::get_agent_present,
            commands::get_active_targets,
            commands::get_pending_ui_workbench_url,
            commands::list_ui_workbench_pages,
            commands::select_ui_workbench_page,
            commands::load_ui_workbench_page,
            commands::read_ui_workbench_asset,
            commands::open_ui_workbench,
            commands::list_ui_workflow_tasks,
            commands::select_ui_workflow_task,
            commands::update_ui_workflow_target,
            commands::search_widget_blueprints,
            commands::preflight_ui_delivery,
            commands::open_ui_workflow,
            commands::start_ui_source_task,
            commands::submit_codex_ui_source_prompt,
            commands::confirm_and_deliver_ui,
            commands::submit_codex_new_task_prompt,
            commands::open_settings,
            commands::open_agent_settings,
            commands::close_settings,
            commands::get_skill_status,
            commands::refresh_skill_status_cmd,
            commands::reinstall_skill,
            commands::reinstall_skill_for_target,
            commands::set_pause_detection,
            commands::get_autostart_enabled,
            commands::set_autostart_enabled,
            commands::launch_agent,
            commands::check_updates,
            commands::install_latest_update,
            commands::check_mcp_status,
            commands::connect_mcp_auto,
            commands::disable_mcp,
            commands::discover_mcp,
            commands::list_mcp_tools,
            commands::call_mcp_tool,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

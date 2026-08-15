//! System tray: icon, context menu, and click handling.

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, Position, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, Window, WindowEvent,
};

const SETTINGS_WIDTH: f64 = 560.0;
const SETTINGS_HEIGHT: f64 = 680.0;
const BALL_WINDOW_SIZE: f64 = 64.0;
const BALL_CORE_SIZE: f64 = 52.0;
const POPOVER_GAP: f64 = 0.0;
const FOLLOW_INTERVAL_MS: u64 = 33;
const POSITION_EPSILON: i32 = 1;

#[derive(Clone, Copy)]
struct PopoverPlacement {
    side: &'static str,
}

/// Build the tray icon and wire its menu + click events.
pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let show_settings_item =
        MenuItem::with_id(app, "open_settings", "打开设置", true, None::<&str>)?;
    let pause_item = MenuItem::with_id(
        app,
        "toggle_pause",
        "暂停 / 恢复 Agent 检测",
        true,
        None::<&str>,
    )?;
    let refresh_skill =
        MenuItem::with_id(app, "refresh_skill", "检查 Skill 状态", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &show_settings_item,
            &pause_item,
            &refresh_skill,
            &sep1,
            &quit,
        ],
    )?;

    let _tray = TrayIconBuilder::with_id("main-tray")
        .icon(
            app.default_window_icon()
                .expect("missing default window icon")
                .clone(),
        )
        .tooltip("Oasis Companion")
        .menu(&menu)
        .on_menu_event(on_menu_event)
        .on_tray_icon_event(on_tray_icon_event)
        .build(app)?;

    Ok(())
}

fn on_menu_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    match event.id().as_ref() {
        "open_settings" => show_settings(app),
        "toggle_pause" => {
            let cur = app
                .state::<crate::state::AppState>()
                .settings
                .lock()
                .unwrap()
                .companion
                .pause_detection;
            if let Err(err) = crate::apply_pause(app, !cur) {
                log::error!("failed to toggle agent detection: {}", err);
            }
        }
        "refresh_skill" => crate::refresh_skill_status(app),
        "quit" => app.exit(0),
        _ => {}
    }
}

fn on_tray_icon_event(tray: &tauri::tray::TrayIcon, event: TrayIconEvent) {
    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
    } = event
    {
        show_settings(tray.app_handle());
    }
}

/// Show (and focus) the settings window.
pub fn show_settings(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("settings") {
        let placement = position_settings_near_ball(app, &w);
        emit_popover_side(&w, placement.map(|p| p.side));
        let _ = w.emit("settings://show-home", ());
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    } else {
        match WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("index.html".into()))
            .title("Oasis Companion 设置")
            .inner_size(SETTINGS_WIDTH, SETTINGS_HEIGHT)
            .min_inner_size(420.0, 560.0)
            .resizable(false)
            .decorations(false)
            .transparent(true)
            .shadow(false)
            .skip_taskbar(true)
            .always_on_top(true)
            .build()
        {
            Ok(w) => {
                let placement = position_settings_near_ball(app, &w);
                emit_popover_side(&w, placement.map(|p| p.side));
                let _ = w.show();
                let _ = w.set_focus();
            }
            Err(err) => log::warn!(
                "settings window not found and could not be created: {}",
                err
            ),
        }
    }
}

pub fn show_agent_settings(app: &AppHandle, target_id: &str) {
    let label = agent_settings_label(target_id);
    let title = format!("Oasis Companion - {}", agent_settings_title(target_id));
    if let Some(w) = app.get_webview_window(&label) {
        let placement = position_settings_near_ball(app, &w);
        emit_popover_side(&w, placement.map(|p| p.side));
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
        return;
    }

    match WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))
        .title(&title)
        .inner_size(SETTINGS_WIDTH, SETTINGS_HEIGHT)
        .min_inner_size(420.0, 560.0)
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .skip_taskbar(true)
        .always_on_top(true)
        .build()
    {
        Ok(w) => {
            let placement = position_settings_near_ball(app, &w);
            emit_popover_side(&w, placement.map(|p| p.side));
            if let Err(err) = w.show() {
                log::warn!("agent settings window could not be shown: {}", err);
            }
            if let Err(err) = w.set_focus() {
                log::warn!("agent settings window could not be focused: {}", err);
            }
        }
        Err(err) => log::warn!("agent settings window could not be created: {}", err),
    }
}

pub fn toggle_settings(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("settings") {
        if w.is_visible().unwrap_or(false) {
            let _ = w.hide();
            return;
        }
    }
    show_settings(app);
}

pub fn hide_settings(app: &AppHandle) {
    for (label, w) in app.webview_windows() {
        if label == "settings" || label.starts_with("settings-") {
            let _ = w.hide();
        }
    }
}

pub fn handle_window_event(window: &Window, event: &WindowEvent) {
    if !should_hide_settings_window(window.label(), event) {
        return;
    }
    if let Err(error) = window.hide() {
        log::warn!(
            "settings window {} could not be hidden after losing focus: {}",
            window.label(),
            error
        );
    }
}

fn should_hide_settings_window(label: &str, event: &WindowEvent) -> bool {
    (label == "settings" || label.starts_with("settings-"))
        && matches!(event, WindowEvent::Focused(false))
}

pub fn spawn_settings_popover_follow_loop(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut last_side: Option<&'static str> = None;
        loop {
            sync_settings_popover(&app, &mut last_side);
            tokio::time::sleep(std::time::Duration::from_millis(FOLLOW_INTERVAL_MS)).await;
        }
    });
}

fn sync_settings_popover(app: &AppHandle, last_side: &mut Option<&'static str>) {
    let Some(settings) = app.get_webview_window("settings") else {
        return;
    };
    if !settings.is_visible().unwrap_or(false) {
        *last_side = None;
        return;
    }
    if let Some(placement) = position_settings_near_ball(app, &settings) {
        if last_side.as_ref().copied() != Some(placement.side) {
            emit_popover_side(&settings, Some(placement.side));
            *last_side = Some(placement.side);
        }
    }
}

fn emit_popover_side(settings: &WebviewWindow, side: Option<&'static str>) {
    let _ = settings.emit("settings://popover-side", side.unwrap_or("floating"));
}

fn agent_settings_label(target_id: &str) -> String {
    format!("settings-{}", target_id)
}

fn agent_settings_title(target_id: &str) -> &'static str {
    match target_id {
        "codex" => "Codex",
        "claude-code" => "Claude Code",
        "workbuddy" => "WorkBuddy",
        _ => "Agent",
    }
}

fn position_settings_near_ball(
    app: &AppHandle,
    settings: &WebviewWindow,
) -> Option<PopoverPlacement> {
    let ball = app.get_webview_window("ball")?;
    if !ball.is_visible().unwrap_or(false) {
        return None;
    }

    let ball_pos = match ball.outer_position() {
        Ok(pos) => pos,
        Err(err) => {
            log::warn!("could not read ball position: {}", err);
            return None;
        }
    };
    let ball_size = match ball.outer_size() {
        Ok(size) => size,
        Err(err) => {
            log::warn!("could not read ball size: {}", err);
            return None;
        }
    };

    let monitor = ball
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten());
    let monitor = monitor?;

    let scale = monitor.scale_factor();
    let gap = (POPOVER_GAP * scale).round() as i32;
    let logo_inset = (((BALL_WINDOW_SIZE - BALL_CORE_SIZE) / 2.0) * scale).round() as i32;
    let fallback_w = (SETTINGS_WIDTH * scale).round() as u32;
    let fallback_h = (SETTINGS_HEIGHT * scale).round() as u32;
    let settings_size = settings
        .outer_size()
        .unwrap_or_else(|_| tauri::PhysicalSize::new(fallback_w, fallback_h));

    let monitor_pos = monitor.position();
    let monitor_size = monitor.size();
    let min_x = monitor_pos.x + gap;
    let min_y = monitor_pos.y + gap;
    let max_x = monitor_pos.x + monitor_size.width as i32 - settings_size.width as i32 - gap;
    let max_y = monitor_pos.y + monitor_size.height as i32 - settings_size.height as i32 - gap;

    let logo_left = ball_pos.x + logo_inset;
    let logo_right = ball_pos.x + ball_size.width as i32 - logo_inset;
    let right_space = monitor_pos.x + monitor_size.width as i32 - logo_right;
    let left_space = logo_left - monitor_pos.x;
    let prefer_right = right_space >= settings_size.width as i32 + gap || right_space >= left_space;

    let raw_x = if prefer_right {
        logo_right + gap
    } else {
        logo_left - settings_size.width as i32 - gap
    };
    let raw_y = ball_pos.y + ball_size.height as i32 / 2 - settings_size.height as i32 / 2;

    let x = clamp_i32(raw_x, min_x, max_x);
    let y = clamp_i32(raw_y, min_y, max_y);
    let side = if prefer_right { "right" } else { "left" };
    if should_move(settings, x, y) {
        let _ = settings.set_position(Position::Physical(PhysicalPosition::new(x, y)));
    }
    Some(PopoverPlacement { side })
}

fn should_move(settings: &WebviewWindow, target_x: i32, target_y: i32) -> bool {
    let Ok(current) = settings.outer_position() else {
        return true;
    };
    (current.x - target_x).abs() > POSITION_EPSILON
        || (current.y - target_y).abs() > POSITION_EPSILON
}

fn clamp_i32(value: i32, min: i32, max: i32) -> i32 {
    if min > max {
        return min;
    }
    value.max(min).min(max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri::WindowEvent;

    #[test]
    fn settings_windows_hide_only_when_they_lose_focus() {
        assert!(should_hide_settings_window(
            "settings",
            &WindowEvent::Focused(false)
        ));
        assert!(should_hide_settings_window(
            "settings-codex",
            &WindowEvent::Focused(false)
        ));
        assert!(!should_hide_settings_window(
            "settings",
            &WindowEvent::Focused(true)
        ));
        assert!(!should_hide_settings_window(
            "ball",
            &WindowEvent::Focused(false)
        ));
        assert!(!should_hide_settings_window(
            "ui-workbench",
            &WindowEvent::Focused(false)
        ));
    }
}

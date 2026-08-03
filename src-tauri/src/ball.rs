//! Floating-ball window lifecycle: show/hide, position, and state broadcast.

use tauri::{AppHandle, Emitter, LogicalPosition, Manager, Position, WebviewWindow};

use crate::state::{AppState, BallState};

/// Ball window size (logical px). Must match `tauri.conf.json` window config.
const BALL_SIZE: f64 = 64.0;
/// Margin from the screen edge.
const EDGE_MARGIN: f64 = 16.0;

/// Apply a new ball state: show/hide the window and notify the frontend.
pub fn apply_state(app: &AppHandle, state: BallState) {
    let ball = match app.get_webview_window("ball") {
        Some(w) => w,
        None => {
            log::warn!("ball window not found");
            return;
        }
    };

    match state {
        BallState::Hidden => {
            let _ = ball.hide();
        }
        _ => {
            position_ball(app, &ball);
            let _ = ball.show();
        }
    }

    let _ = app.emit("ball://state", state);
    log::info!("ball state -> {}", state.label());
}

/// Place the ball at the saved position, or auto-anchor to the right-center of
/// the primary monitor. (Position persistence is deferred to a later phase;
/// for now it always re-anchors on show.)
fn position_ball(app: &AppHandle, ball: &WebviewWindow) {
    // Honor a user-saved position if present.
    {
        let st = app.state::<AppState>();
        let s = st.settings.lock().unwrap();
        if let (Some(x), Some(y)) = (s.ball.position.x, s.ball.position.y) {
            let _ = ball.set_position(Position::Logical(LogicalPosition::new(x, y)));
            return;
        }
    }

    // Otherwise anchor to the right-center of the primary monitor.
    let monitor = match app.primary_monitor() {
        Ok(Some(m)) => m,
        _ => return,
    };
    let scale = monitor.scale_factor();
    let mw = monitor.size().width as f64 / scale;
    let mh = monitor.size().height as f64 / scale;
    let mx = monitor.position().x as f64 / scale;
    let my = monitor.position().y as f64 / scale;

    let x = mx + mw - BALL_SIZE - EDGE_MARGIN;
    let y = my + (mh - BALL_SIZE) / 2.0;
    let _ = ball.set_position(Position::Logical(LogicalPosition::new(x, y)));
}

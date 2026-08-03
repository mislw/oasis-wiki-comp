//! Window-title-based Agent detection (Windows, via `EnumWindows`).
//!
//! STUB for MVP — process-name detection (`process.rs`) is the primary method
//! and covers Codex/ChatGPT. Window-title matching is reserved for cases where
//! the Agent runs inside a host process (e.g. a terminal) and only its window
//! title is distinguishable. Implement using the `windows` crate's
//! `EnumWindows` + `GetWindowTextW` when needed.

/// Not implemented in MVP. Always returns false.
pub fn any_window_matches(_titles: &[String]) -> bool {
    // TODO(mvp+1): enumerate top-level windows and substring-match titles.
    false
}

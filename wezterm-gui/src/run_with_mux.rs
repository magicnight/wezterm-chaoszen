//! Shared GUI event-loop body for wezterm-gui binary and chaoszen-app.
//!
//! Slice 1b.4.a — extracted from `main.rs::run_terminal_gui` so that
//! both the wezterm-gui binary (LocalDomain path) and chaoszen-app
//! (zellij-backend path) drive the same frontend startup +
//! event loop. See spec at
//! `docs/superpowers/specs/2026-05-10-slice-1b4a-wezterm-gui-modular-entry-design.md`.

/// Subset of CLI / config options that the GUI shared body needs to
/// configure the window before frontend init. Caller-specific data
/// (CommandBuilder, workspace, domain) lives inside the `SetupTask`
/// closure instead of this struct.
pub struct RunWithMuxOpts {
    /// Window class for X11/Wayland; None → wezterm built-in default.
    pub class: Option<String>,
    /// Initial window position; None → frontend default placement.
    pub position: Option<config::GuiPosition>,
}

impl Default for RunWithMuxOpts {
    fn default() -> Self {
        Self {
            class: None,
            position: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_with_mux_opts_defaults() {
        let opts = RunWithMuxOpts::default();
        assert!(opts.class.is_none(), "class default must be None");
        assert!(opts.position.is_none(), "position default must be None");
    }
}

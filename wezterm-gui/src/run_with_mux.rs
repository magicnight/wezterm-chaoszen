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

use std::future::Future;
use std::pin::Pin;

/// One-shot async setup task: a closure that runs once after frontend
/// initialization to populate the Mux's windows/tabs from caller's
/// vantage. wezterm-gui binary passes a closure that calls
/// `async_run_terminal_gui`; chaoszen-app (1b.4.b) will pass a
/// closure that attaches the existing zellij Tab to a Window.
pub struct SetupTask {
    inner: Box<
        dyn FnOnce() -> Pin<Box<dyn Future<Output = anyhow::Result<()>>>> + Send,
    >,
}

impl SetupTask {
    pub fn new<F, Fut>(f: F) -> Self
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = anyhow::Result<()>> + 'static,
    {
        Self {
            inner: Box::new(move || Box::pin(f())),
        }
    }

    /// Internal: consume the task and produce the boxed future that
    /// `run_with_mux` will spawn via `promise::spawn::spawn`.
    pub(crate) fn into_future(
        self,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<()>>>> {
        (self.inner)()
    }
}

/// Run the wezterm GUI event loop against a pre-prepared Mux.
///
/// Applies window class / position from `opts`, initializes the GUI
/// frontend (`frontend::try_new`), spawns the caller's `setup` task
/// via `promise::spawn::spawn`, shows any deferred configuration
/// errors, then drives the event loop until the window closes / app
/// exits. This mirrors the tail of the original `run_terminal_gui`
/// (see `wezterm-gui/src/lib.rs`), now shared between the wezterm-gui
/// binary (LocalDomain path) and chaoszen-app (zellij-backend path).
///
/// Caller responsibilities:
///   - Mux already constructed and registered via `Mux::set_mux(&mux)`.
///   - At least one Domain registered on the Mux.
///   - Configuration system already initialized.
pub fn run_with_mux(
    _mux: std::sync::Arc<mux::Mux>,
    _config: config::ConfigHandle,
    opts: RunWithMuxOpts,
    setup: SetupTask,
) -> anyhow::Result<()> {
    if let Some(class) = opts.class.as_deref() {
        crate::termwindow::set_window_class(class);
    }
    if let Some(pos) = opts.position {
        crate::termwindow::set_window_position(pos);
    }

    let gui = crate::frontend::try_new()?;
    let activity = mux::activity::Activity::new();

    promise::spawn::spawn(async move {
        if let Err(err) = setup.into_future().await {
            crate::terminate_with_error(err);
        }
        drop(activity);
    })
    .detach();

    crate::maybe_show_configuration_error_window();
    gui.run_forever()
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

    #[test]
    fn setup_task_roundtrip() {
        // Construct, into_future, await — confirms the closure plumbing
        // is wired correctly without depending on the GUI frontend.
        use std::sync::atomic::{AtomicBool, Ordering};
        let ran = std::sync::Arc::new(AtomicBool::new(false));
        let ran_clone = ran.clone();
        let task = SetupTask::new(move || {
            let ran = ran_clone;
            async move {
                ran.store(true, Ordering::SeqCst);
                Ok(())
            }
        });
        let fut = task.into_future();
        // Block on the future (no async runtime needed for this trivial body)
        smol::block_on(fut).expect("setup task must succeed");
        assert!(ran.load(Ordering::SeqCst), "setup task body did not run");
    }
}

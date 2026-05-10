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
/// **Slice 1b.4.a SHELL ONLY** — this function currently returns an
/// error. The real body (frontend init + spawn + run_forever) is
/// deferred to Slice 1b.4.b which will perform the deep restructure
/// needed to move helpers (`set_window_class`, `frontend::try_new`,
/// `Activity`, `terminate_with_error`, `maybe_show_configuration_error_window`)
/// from `wezterm-gui/src/main.rs` (binary crate) into the
/// `wezterm-gui` library crate so this lib-side function can call
/// them.
///
/// Caller responsibilities (when 1b.4.b unblocks the body):
///   - Mux already constructed and registered via `Mux::set_mux(&mux)`.
///   - At least one Domain registered on the Mux.
///   - Configuration system already initialized.
///
/// What `run_with_mux` will do (1b.4.b):
///   1. Apply window class / position from `opts`.
///   2. Initialize the GUI frontend (`frontend::try_new`).
///   3. Spawn the caller's `setup` task via `promise::spawn::spawn`.
///   4. Show any deferred configuration errors.
///   5. Run the event loop until the window closes / app exits.
pub fn run_with_mux(
    _mux: std::sync::Arc<mux::Mux>,
    _config: config::ConfigHandle,
    _opts: RunWithMuxOpts,
    _setup: SetupTask,
) -> anyhow::Result<()> {
    anyhow::bail!(
        "wezterm_gui::run_with_mux body deferred to Slice 1b.4.b; see \
         docs/superpowers/specs/2026-05-10-slice-1b4a-wezterm-gui-modular-entry-design.md"
    )
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

    #[test]
    fn run_with_mux_stub_bails() {
        // 1b.4.a deliverable: the function exists and is callable.
        // Its body is a stub bail!() until 1b.4.b's restructure lands.
        // This test pins the contract: callers can construct the args
        // and invoke the function; verification of the actual GUI
        // behavior moves to 1b.4.b's integration smoke.
        let mux = std::sync::Arc::new(mux::Mux::new(None));
        let config = config::configuration();
        let opts = RunWithMuxOpts::default();
        let setup = SetupTask::new(|| async { Ok(()) });
        let err = run_with_mux(mux, config, opts, setup)
            .expect_err("stub must return Err");
        let msg = format!("{}", err);
        assert!(
            msg.contains("1b.4.b"),
            "error message must reference 1b.4.b, got: {}",
            msg
        );
    }

    /// Compile-time verification that 1b.4.b.1's migration made the
    /// helpers `run_with_mux`'s body needs reachable from the lib.
    /// Pure type-level assertion — no GUI is started.
    ///
    /// 1b.4.b.2 will USE these symbols inside `run_with_mux`'s body
    /// and this test becomes redundant; delete then.
    #[test]
    fn helpers_reachable_from_lib() {
        let _set_class: fn(&str) = crate::termwindow::set_window_class;
        let _set_pos: fn(config::GuiPosition) = crate::termwindow::set_window_position;
        let _terminate: fn(anyhow::Error) -> ! = crate::terminate_with_error;
        let _activity_new: fn() -> mux::activity::Activity = mux::activity::Activity::new;
    }
}

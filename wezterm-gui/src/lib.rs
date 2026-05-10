//! wezterm-gui library crate.
//!
//! Slice 1b.4.a: extracted from binary-only crate so chaoszen-app
//! (1b.4.b) can call `run_with_mux` instead of going through the
//! `wezterm-gui` CLI binary.
//!
//! Public API: `run_with_mux`, `RunWithMuxOpts`, `SetupTask`.
//! All other modules remain `pub(crate)` and are not exposed
//! externally.

// Re-exports added in Task 4 once run_with_mux module exists.

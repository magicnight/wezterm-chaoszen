//! wezterm-gui library crate.
//!
//! Slice 1b.4.a: extracted from binary-only crate so chaoszen-app
//! (1b.4.b) can call `run_with_mux` instead of going through the
//! `wezterm-gui` CLI binary.
//!
//! Public API: `run_with_mux`, `RunWithMuxOpts`, `SetupTask`.
//! All other modules remain `pub(crate)` and are not exposed
//! externally.
//!
//! NOTE 1b.4.a: `run_with_mux`'s body is currently stubbed — it
//! returns an error. 1b.4.b moves the GUI event-loop helpers from
//! `main.rs` into this crate so the body can be implemented.

pub mod run_with_mux;
pub use run_with_mux::{run_with_mux, RunWithMuxOpts, SetupTask};

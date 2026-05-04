//! Zellij-backed Mux backbone.
//!
//! Holds the `zellij_server::embedded::ServerHandle` (used by `LocalDomain`
//! in 1b.3 to send instructions) and the `JoinHandle` of the outbound drain
//! thread (which reads `ServerToClientMsg`s from zellij-server and feeds
//! them into Mux state).
//!
//! Slice 1b.2 is shallow: the drain thread only logs message discriminants.
//! Slice 1b.3 will populate `Mux::tabs / panes / windows` from `Render` /
//! `KillSession` / `SwitchTab` events.

use std::thread::JoinHandle;

use zellij_server::embedded::ServerHandle;

pub struct ZellijBackend {
    pub handle: ServerHandle,
    /// Outbound drain thread handle. Dropping `ZellijBackend` does NOT join
    /// the thread; the thread exits when the outbound channel disconnects
    /// (which happens when `start_session_blocking` returns or
    /// zellij-server's main loop hits `KillSession`).
    pub drain_join: JoinHandle<()>,
}

impl std::fmt::Debug for ZellijBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ZellijBackend").finish_non_exhaustive()
    }
}

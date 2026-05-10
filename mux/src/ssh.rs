//! Stub for the deleted ssh module (1b.1 removed wezterm's ssh
//! remote-mux). chaoszen retires the wezterm mux protocol entirely
//! (decision #5); ssh callers in `wezterm-mux-server-impl` and
//! similar dead crates compile against these no-op signatures
//! until a future cleanup slice removes them.
//!
//! Pre-1b.1, this module hosted `RemoteSshDomain` (a `Domain` impl
//! tunneled over wezterm-ssh) plus `ssh_connect_with_ui` and
//! `ssh_domain_to_ssh_config` helpers. None of those work in
//! chaoszen, but the call sites still exist; they all return errors
//! rather than panicking so the binaries compile and degrade
//! gracefully.

use crate::domain::{alloc_domain_id, Domain, DomainId, DomainState, SplitSource};
use crate::pane::{Pane, PaneId};
use crate::tab::{SplitRequest, Tab, TabId};
use crate::window::WindowId;
use async_trait::async_trait;
use portable_pty::CommandBuilder;
use std::sync::Arc;
use wezterm_term::TerminalSize;

/// Stub for the pre-1b.1 `RemoteSshDomain` — a `Domain` implementation
/// that tunneled wezterm-mux PDUs over an ssh transport. chaoszen
/// retires the wezterm mux protocol; constructing or driving this
/// domain always errors.
pub struct RemoteSshDomain {
    name: String,
    id: DomainId,
}

impl RemoteSshDomain {
    /// Pre-1b.1: built a `RemoteSshDomain` from a configured
    /// `SshDomain`. chaoszen has no remote ssh support; the resulting
    /// domain is constructed but every operation bails.
    pub fn with_ssh_domain(dom: &config::SshDomain) -> anyhow::Result<Self> {
        Ok(Self {
            name: dom.name.clone(),
            id: alloc_domain_id(),
        })
    }
}

#[async_trait(?Send)]
impl Domain for RemoteSshDomain {
    async fn spawn_pane(
        &self,
        _size: TerminalSize,
        _command: Option<CommandBuilder>,
        _command_dir: Option<String>,
    ) -> anyhow::Result<Arc<dyn Pane>> {
        anyhow::bail!("ssh remote-mux removed in chaoszen; see slice 1b.1")
    }

    fn domain_id(&self) -> DomainId {
        self.id
    }

    fn domain_name(&self) -> &str {
        &self.name
    }

    async fn attach(&self, _window_id: Option<WindowId>) -> anyhow::Result<()> {
        anyhow::bail!("ssh remote-mux removed in chaoszen; see slice 1b.1")
    }

    fn detachable(&self) -> bool {
        false
    }

    fn detach(&self) -> anyhow::Result<()> {
        anyhow::bail!("ssh remote-mux removed in chaoszen; see slice 1b.1")
    }

    fn state(&self) -> DomainState {
        DomainState::Detached
    }
}

/// Stub for `ssh_connect_with_ui` — original returned a wezterm-ssh
/// `Session`. Always errors in chaoszen.
pub fn ssh_connect_with_ui<U>(
    _config: (),
    _ui: U,
) -> anyhow::Result<()> {
    anyhow::bail!("ssh remote-mux removed in chaoszen; see slice 1b.1")
}

/// Stub for `ssh_domain_to_ssh_config` — original returned a
/// wezterm-ssh `Config`. Always errors in chaoszen.
pub fn ssh_domain_to_ssh_config<D>(_dom: &D) -> anyhow::Result<()> {
    anyhow::bail!("ssh remote-mux removed in chaoszen; see slice 1b.1")
}

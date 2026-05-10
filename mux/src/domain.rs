//! A Domain represents an instance of a multiplexer.
//!
//! In the chaoszen fork the only relevant domain is the local one
//! (eventually backed by zellij-server in Slice 1b.2). Upstream
//! WezTerm also supports remote ssh and wezterm-mux-server domains;
//! those have been removed (Slice 1b.1) along with the underlying
//! ssh / tmux multiplexer code.

use crate::pane::{Pane, PaneId};
use crate::tab::{SplitRequest, Tab, TabId};
use crate::window::WindowId;
use crate::Mux;
use anyhow::Context;
use async_trait::async_trait;
use downcast_rs::{impl_downcast, Downcast};
use portable_pty::CommandBuilder;
use std::sync::Arc;
use wezterm_term::TerminalSize;

static DOMAIN_ID: ::std::sync::atomic::AtomicUsize = ::std::sync::atomic::AtomicUsize::new(0);
pub type DomainId = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainState {
    Detached,
    Attached,
}

pub fn alloc_domain_id() -> DomainId {
    DOMAIN_ID.fetch_add(1, ::std::sync::atomic::Ordering::Relaxed)
}

#[derive(Debug, Clone, PartialEq)]
pub enum SplitSource {
    Spawn {
        command: Option<CommandBuilder>,
        command_dir: Option<String>,
    },
    MovePane(PaneId),
}

#[async_trait(?Send)]
pub trait Domain: Downcast + Send + Sync {
    /// Spawn a new command within this domain
    async fn spawn(
        &self,
        size: TerminalSize,
        command: Option<CommandBuilder>,
        command_dir: Option<String>,
        window: WindowId,
    ) -> anyhow::Result<Arc<Tab>> {
        let pane = self
            .spawn_pane(size, command, command_dir)
            .await
            .context("spawn")?;

        let tab = Arc::new(Tab::new_orphan());
        tab.assign_pane(&pane);

        let mux = Mux::get();
        mux.add_tab_and_active_pane(&tab)?;
        mux.add_tab_to_window(&tab, window)?;

        Ok(tab)
    }

    async fn split_pane(
        &self,
        source: SplitSource,
        tab: TabId,
        pane_id: PaneId,
        split_request: SplitRequest,
    ) -> anyhow::Result<Arc<dyn Pane>> {
        let mux = Mux::get();
        let tab = match mux.get_tab(tab) {
            Some(t) => t,
            None => anyhow::bail!("Invalid tab id {}", tab),
        };

        let pane_index = match tab
            .iter_panes_ignoring_zoom()
            .iter()
            .find(|p| p.pane.pane_id() == pane_id)
        {
            Some(p) => p.index,
            None => anyhow::bail!("invalid pane id {}", pane_id),
        };

        let split_size = match tab.compute_split_size(pane_index, split_request) {
            Some(s) => s,
            None => anyhow::bail!("invalid pane index {}", pane_index),
        };

        // 1b.3.c stub: convert PaneSize → TerminalSize for spawn_pane signature.
        // Real geometry computation happens in 1b.3.c when zellij drives layout.
        let second_size = TerminalSize {
            rows: split_size.second.rows,
            cols: split_size.second.cols,
            pixel_width: split_size.second.pixel_width,
            pixel_height: split_size.second.pixel_height,
            dpi: 0,
        };

        let pane = match source {
            SplitSource::Spawn {
                command,
                command_dir,
            } => {
                self.spawn_pane(second_size, command, command_dir)
                    .await?
            }
            SplitSource::MovePane(src_pane_id) => {
                let (_domain, _window, src_tab) = mux
                    .resolve_pane_id(src_pane_id)
                    .ok_or_else(|| anyhow::anyhow!("pane {} not found", src_pane_id))?;
                let src_tab = match mux.get_tab(src_tab) {
                    Some(t) => t,
                    None => anyhow::bail!("Invalid tab id {}", src_tab),
                };

                let pane = src_tab.remove_pane(src_pane_id).ok_or_else(|| {
                    anyhow::anyhow!("pane {} not found in its containing tab!?", src_pane_id)
                })?;

                if src_tab.is_dead() {
                    mux.remove_tab(src_tab.tab_id());
                }

                pane
            }
        };

        // pane_index may have changed if src_pane was also in the same tab
        let final_pane_index = match tab
            .iter_panes_ignoring_zoom()
            .iter()
            .find(|p| p.pane.pane_id() == pane_id)
        {
            Some(p) => p.index,
            None => anyhow::bail!("invalid pane id {}", pane_id),
        };

        tab.split_and_insert(final_pane_index, split_request, Arc::clone(&pane))?;
        Ok(pane)
    }

    async fn spawn_pane(
        &self,
        size: TerminalSize,
        command: Option<CommandBuilder>,
        command_dir: Option<String>,
    ) -> anyhow::Result<Arc<dyn Pane>>;

    /// The mux will call this method on the domain of the pane that
    /// is being moved to give the domain a chance to handle the movement.
    /// If this method returns Ok(None), then the mux will handle the
    /// movement itself by mutating its local Tabs and Windows.
    async fn move_pane_to_new_tab(
        &self,
        _pane_id: PaneId,
        _window_id: Option<WindowId>,
        _workspace_for_new_window: Option<String>,
    ) -> anyhow::Result<Option<(Arc<Tab>, WindowId)>> {
        Ok(None)
    }

    /// Returns false if the `spawn` method will never succeed.
    /// There are some internal placeholder domains that are
    /// pre-created with local UI that we do not want to allow
    /// to show in the launcher/menu as launchable items.
    fn spawnable(&self) -> bool {
        true
    }

    /// Returns true if the `detach` method can be used
    /// to detach the domain, preserving the associated
    /// panes, or false if the `detach` method will never
    /// succeed
    fn detachable(&self) -> bool;

    /// Returns the domain id, which is useful for obtaining
    /// a handle on the domain later.
    fn domain_id(&self) -> DomainId;

    /// Returns the name of the domain.
    /// Should be a short identifier.
    fn domain_name(&self) -> &str;

    /// Returns a label describing the domain.
    async fn domain_label(&self) -> String {
        self.domain_name().to_string()
    }

    /// Re-attach to any tabs that might be pre-existing in this domain
    async fn attach(&self, window_id: Option<WindowId>) -> anyhow::Result<()>;

    /// Detach all tabs
    fn detach(&self) -> anyhow::Result<()>;

    /// Indicates the state of the domain
    fn state(&self) -> DomainState;
}
impl_downcast!(Domain);

pub struct LocalDomain {
    name: String,
    id: DomainId,
    state: parking_lot::RwLock<DomainState>,
}

impl LocalDomain {
    pub fn new(name: &str) -> anyhow::Result<Self> {
        Ok(Self {
            name: name.to_string(),
            id: alloc_domain_id(),
            state: parking_lot::RwLock::new(DomainState::Detached),
        })
    }

    /// Pre-chaoszen: spawned a WSL-backed local domain. chaoszen does
    /// not support WSL domains (Windows is shipped via the binary
    /// directly, not via wezterm's WSL bridging).
    pub fn new_wsl(_dom: config::WslDomain) -> anyhow::Result<Self> {
        anyhow::bail!("WSL domains unsupported in chaoszen; see decision #6")
    }

    /// Pre-chaoszen: spawned an exec-domain (custom shell wrapper).
    /// chaoszen retires this; use a regular shell or the AI bridge.
    pub fn new_exec_domain(_dom: config::ExecDomain) -> anyhow::Result<Self> {
        anyhow::bail!("exec domains unsupported in chaoszen; see decision #6")
    }

    /// Pre-chaoszen: spawned a serial-port-backed domain. chaoszen does
    /// not support serial domains (out of scope per master plan).
    pub fn new_serial_domain(_dom: config::SerialDomain) -> anyhow::Result<Self> {
        anyhow::bail!("serial domains unsupported in chaoszen; see decision #6")
    }
}

#[async_trait(?Send)]
impl Domain for LocalDomain {
    async fn spawn_pane(
        &self,
        _size: TerminalSize,
        _command: Option<CommandBuilder>,
        _command_dir: Option<String>,
    ) -> anyhow::Result<Arc<dyn Pane>> {
        anyhow::bail!(
            "LocalDomain::spawn_pane not implemented in Slice 1b.2;              see Slice 1b.3 (Tab + Pane rewrite) for zellij-backed implementation"
        );
    }

    fn domain_id(&self) -> DomainId {
        self.id
    }

    fn domain_name(&self) -> &str {
        &self.name
    }

    async fn attach(&self, _window_id: Option<WindowId>) -> anyhow::Result<()> {
        anyhow::bail!("LocalDomain::attach not implemented in Slice 1b.2; see Slice 1b.3");
    }

    fn detachable(&self) -> bool {
        false
    }

    fn detach(&self) -> anyhow::Result<()> {
        anyhow::bail!("LocalDomain::detach not implemented in Slice 1b.2; see Slice 1b.3");
    }

    fn state(&self) -> DomainState {
        *self.state.read()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_domain_spawn_pane_returns_unimplemented_error() {
        let domain = LocalDomain::new("test").expect("LocalDomain::new");
        let result = smol::block_on(domain.spawn_pane(
            TerminalSize::default(),
            None,
            None,
        ));
        let err = match result {
            Ok(_) => panic!("expected Err, got Ok"),
            Err(e) => e,
        };
        let s = err.to_string();
        assert!(
            s.contains("1b.3"),
            "{}", "expected '1b.3' in error message, got: {s}"
        );
    }

    #[test]
    fn local_domain_basic_getters() {
        let domain = LocalDomain::new("hello").unwrap();
        assert_eq!(domain.domain_name(), "hello");
        assert!(!domain.detachable());
        assert!(matches!(domain.state(), DomainState::Detached));
    }
}

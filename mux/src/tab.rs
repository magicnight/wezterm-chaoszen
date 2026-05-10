//! Slice 1b.3.a thin wrapper of zellij's tab id.
//!
//! Pre-1b.3.a, this file held 2528 LOC of split-tree logic (active
//! pane tracking, zoom, layout, pane traversal). chaoszen delegates
//! all of that to zellij-server (which holds the authoritative pane
//! tree).
//!
//! 1b.3.c will fill in the wezterm-gui-facing methods that need to
//! work via ServerHandle. 1b.3.a only preserves the type signatures
//! that mux's internal modules import (per Task 7 inventory:
//! domain.rs, lib.rs, window.rs, termwiztermtab.rs, localpane.rs).

use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use wezterm_term::TerminalSize;

use crate::domain::DomainId;
use crate::pane::{CloseReason, Pane, PaneId};

pub type TabId = usize;

static TAB_ID: ::std::sync::atomic::AtomicUsize = ::std::sync::atomic::AtomicUsize::new(0);

pub fn alloc_tab_id() -> TabId {
    TAB_ID.fetch_add(1, ::std::sync::atomic::Ordering::Relaxed)
}

pub struct Tab {
    tab_id: TabId,
    /// Zellij-side tab id. Slice 1b.3.c uses this when forwarding tab
    /// commands to zellij-server.
    zellij_tab_id: u32,
    /// Local cache of pane membership. Populated by RenderCache drain
    /// in 1b.3.b; populated by spawn_pane in 1b.3.c.
    panes: RwLock<Vec<Arc<dyn Pane>>>,
    title: RwLock<String>,
    active_pane_id: RwLock<Option<PaneId>>,
}

impl Tab {
    /// Construct a new Tab bound to the given zellij tab id.
    ///
    /// Accepts either a `u32` (chaoszen callers from `mux::lib`) or a
    /// `&TerminalSize` (legacy `wezterm-client` callers, which had a
    /// pre-1b.3.a `Tab::new(&size)` API). The legacy path binds to
    /// zellij_tab_id=0; a future cleanup slice removes it.
    pub fn new<T: TabConstructorArg>(arg: T) -> Self {
        Self {
            tab_id: alloc_tab_id(),
            zellij_tab_id: arg.into_zellij_tab_id(),
            panes: RwLock::new(Vec::new()),
            title: RwLock::new(String::new()),
            active_pane_id: RwLock::new(None),
        }
    }

    /// Construct a Tab without a zellij tab id (legacy callers like
    /// termwiztermtab that don't have one).
    pub fn new_orphan() -> Self {
        Self::new(0)
    }

    pub fn tab_id(&self) -> TabId {
        self.tab_id
    }

    pub fn zellij_tab_id(&self) -> u32 {
        self.zellij_tab_id
    }

    pub fn assign_pane(&self, pane: &Arc<dyn Pane>) {
        let mut panes = self.panes.write();
        panes.push(Arc::clone(pane));
        // First assigned pane becomes active by default.
        let mut active = self.active_pane_id.write();
        if active.is_none() {
            *active = Some(pane.pane_id());
        }
    }

    pub fn set_active_pane(&self, pane: &Arc<dyn Pane>) {
        *self.active_pane_id.write() = Some(pane.pane_id());
    }

    pub fn get_active_pane(&self) -> Option<Arc<dyn Pane>> {
        let active_id = (*self.active_pane_id.read())?;
        self.panes
            .read()
            .iter()
            .find(|p| p.pane_id() == active_id)
            .cloned()
    }

    pub fn count_panes(&self) -> usize {
        self.panes.read().len()
    }

    pub fn set_title(&self, title: &str) {
        *self.title.write() = title.to_string();
    }

    pub fn get_title(&self) -> String {
        self.title.read().clone()
    }

    pub fn is_dead(&self) -> bool {
        // 1b.3.a: never dead. 1b.3.b: read RenderCache.
        false
    }

    pub fn prune_dead_panes(&self) -> bool {
        // 1b.3.a: no-op (no panes ever marked dead). 1b.3.b: prune via cache.
        false
    }

    pub fn can_close_without_prompting(&self, _reason: CloseReason) -> bool {
        // 1b.3.a: permissive (let close happen). 1b.3.c: consult zellij.
        true
    }

    pub fn iter_panes_ignoring_zoom(&self) -> Vec<PositionedPane> {
        // 1b.3.a: empty. 1b.3.c: build PositionedPane list from panes
        // and the geometry recorded in RenderCache.
        vec![]
    }

    pub fn compute_split_size(
        &self,
        _pane_index: usize,
        _split_request: SplitRequest,
    ) -> Option<SplitDirectionAndSize> {
        // 1b.3.a stub: None. 1b.3.c: real implementation.
        None
    }

    pub fn split_and_insert(
        &self,
        _pane_index: usize,
        _split_request: SplitRequest,
        _pane: Arc<dyn Pane>,
    ) -> anyhow::Result<()> {
        anyhow::bail!(
            "Tab::split_and_insert not implemented in Slice 1b.3.a; see 1b.3.c"
        )
    }

    /// Return the size of the terminal associated with this tab.
    /// 1b.3.a stub: returns default. 1b.3.c: query zellij geometry cache.
    pub fn get_size(&self) -> TerminalSize {
        // 1b.3.c stub
        TerminalSize::default()
    }

    /// Remove a pane from this tab's local pane list and return it.
    /// 1b.3.a stub: always returns None. 1b.3.c: real implementation.
    pub fn remove_pane(&self, pane_id: PaneId) -> Option<Arc<dyn Pane>> {
        // 1b.3.c stub
        let mut panes = self.panes.write();
        if let Some(pos) = panes.iter().position(|p| p.pane_id() == pane_id) {
            Some(panes.remove(pos))
        } else {
            None
        }
    }

    /// Mark all panes belonging to the given domain as dead.
    /// 1b.3.a stub: no-op. 1b.3.c: propagate to zellij.
    pub fn kill_panes_in_domain(&self, _domain: DomainId) {
        // 1b.3.c stub
    }
}

// ====================================================================
// Public types preserved for mux-internal callers.
// wezterm-gui's consumption is fixed in 1b.4.
// ====================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SplitRequest {
    pub direction: SplitDirection,
    pub target_is_second: bool,
    pub size: SplitSize,
    pub top_level: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SplitSize {
    Cells(usize),
    Percent(u8),
}

impl Default for SplitSize {
    fn default() -> Self {
        SplitSize::Percent(50)
    }
}

#[derive(Debug, Clone)]
pub struct SplitDirectionAndSize {
    pub direction: SplitDirection,
    pub first: PaneSize,
    pub second: PaneSize,
}

#[derive(Debug, Clone, Default)]
pub struct PaneSize {
    pub rows: usize,
    pub cols: usize,
    pub pixel_width: usize,
    pub pixel_height: usize,
}

#[derive(Clone)]
pub struct PositionedPane {
    pub index: usize,
    pub pane: Arc<dyn Pane>,
    pub left: usize,
    pub top: usize,
    pub width: usize,
    pub height: usize,
    pub pixel_width: usize,
    pub pixel_height: usize,
    pub is_active: bool,
    pub is_zoomed: bool,
}

impl std::fmt::Debug for PositionedPane {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PositionedPane")
            .field("index", &self.index)
            .field("pane_id", &self.pane.pane_id())
            .field("left", &self.left)
            .field("top", &self.top)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("pixel_width", &self.pixel_width)
            .field("pixel_height", &self.pixel_height)
            .field("is_active", &self.is_active)
            .field("is_zoomed", &self.is_zoomed)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct PositionedSplit {
    pub direction: SplitDirection,
    pub left: usize,
    pub top: usize,
    pub size: usize,
}

// =============================================================================
// 1b.3.a stub re-additions
// =============================================================================
//
// These types and Tab methods were part of pre-1b.3.a wezterm but no longer
// have a meaningful semantics in the chaoszen architecture (zellij-server
// owns the pane tree). They're re-added as stubs to keep `codec`,
// `lua-api-crates/mux`, and `wezterm-client` compiling. A future cleanup
// slice will delete them along with their callers.

/// Pre-1b.3.a tree-pane representation. chaoszen no longer owns the pane
/// tree (zellij-server does), so this is a placeholder for IPC type
/// compatibility only.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PaneNode {
    /// Tab id this pane belongs to (or 0 for the synthetic root).
    pub tab_id: TabId,
}

impl PaneNode {
    /// Pre-1b.3.a method: returned the root TerminalSize of this subtree.
    /// chaoszen IPC doesn't carry pane-tree geometry through PaneNode;
    /// always None.
    pub fn root_size(&self) -> Option<wezterm_term::TerminalSize> {
        None
    }

    /// Pre-1b.3.a method: returned the (window_id, tab_id) for the
    /// first leaf in this subtree. chaoszen IPC doesn't carry these;
    /// always None.
    pub fn window_and_tab_ids(&self) -> Option<(crate::window::WindowId, TabId)> {
        None
    }
}

/// Pre-1b.3.a serialization wrapper around `url::Url`. chaoszen IPC paths
/// don't carry working-dir URLs through this type anymore. Kept as
/// stub for codec compatibility.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SerdeUrl(pub String);

impl std::fmt::Display for SerdeUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<SerdeUrl> for url::Url {
    /// Pre-1b.3.a IPC carried URLs as SerdeUrl. chaoszen doesn't use this
    /// path; the conversion best-effort parses or falls back to about:blank.
    fn from(s: SerdeUrl) -> Self {
        url::Url::parse(&s.0).unwrap_or_else(|_| {
            url::Url::parse("about:blank").expect("about:blank parses")
        })
    }
}

impl From<&url::Url> for SerdeUrl {
    /// Pre-1b.3.a IPC produced SerdeUrl from a `url::Url` reference.
    /// chaoszen retains the conversion for codec callers; the value
    /// is just the stringified URL.
    fn from(u: &url::Url) -> Self {
        SerdeUrl(u.to_string())
    }
}

impl From<url::Url> for SerdeUrl {
    /// Pre-1b.3.a IPC produced SerdeUrl from an owned `url::Url`.
    /// chaoszen retains the conversion for codec callers.
    fn from(u: url::Url) -> Self {
        SerdeUrl(u.to_string())
    }
}

/// Sealed marker trait for `Tab::new` arguments. Pre-1b.3.a callers in
/// `wezterm-client` pass `&TerminalSize`; chaoszen callers in `mux::lib`
/// pass `u32` (the zellij-side tab id). Both compile via this shim until
/// a cleanup slice removes the legacy callers.
pub trait TabConstructorArg {
    fn into_zellij_tab_id(self) -> u32;
}

impl TabConstructorArg for u32 {
    fn into_zellij_tab_id(self) -> u32 {
        self
    }
}

impl TabConstructorArg for &wezterm_term::TerminalSize {
    fn into_zellij_tab_id(self) -> u32 {
        // Pre-1b.3.a: TerminalSize was the tab's pty size. chaoszen doesn't
        // map size -> zellij tab id; a future cleanup slice deletes the
        // wezterm-client caller. For now, bind to 0 (orphan).
        0
    }
}

impl Tab {
    /// Pre-1b.3.a method: returned the pane index reachable in the given
    /// direction from the active pane. chaoszen delegates pane geometry
    /// to zellij-server; always None.
    pub fn get_pane_direction(
        &self,
        _direction: config::keyassignment::PaneDirection,
        _wrap: bool,
    ) -> Option<usize> {
        None
    }

    /// Pre-1b.3.a method: toggled the zoom state of a pane. chaoszen does
    /// not implement pane zoom; always returns false (was-zoomed=false).
    pub fn set_zoomed(&self, _zoomed: bool) -> bool {
        false
    }

    /// Pre-1b.3.a method: rotated panes counter-clockwise within the tab.
    /// chaoszen delegates to zellij; this is a no-op.
    pub fn rotate_counter_clockwise(&self) {}

    /// Pre-1b.3.a method: zoom-honoring pane iteration. chaoszen has no
    /// zoom state; delegates to `iter_panes_ignoring_zoom`.
    pub fn iter_panes(&self) -> Vec<PositionedPane> {
        self.iter_panes_ignoring_zoom()
    }

    /// Pre-1b.3.a method: synchronized this tab's pane tree to a remote
    /// PaneNode. chaoszen does not run the wezterm-mux-server protocol;
    /// always a no-op.
    pub fn sync_with_pane_tree<F>(
        &self,
        _size: wezterm_term::TerminalSize,
        _root: PaneNode,
        _make_pane: F,
    ) where
        F: FnMut(PaneEntry) -> Arc<dyn Pane>,
    {
    }

    /// Pre-1b.3.a method: serialized the pane tree for IPC. chaoszen
    /// IPC doesn't carry pane trees; returns an empty PaneNode.
    pub fn codec_pane_tree(&self) -> PaneNode {
        PaneNode::default()
    }

    /// Pre-1b.3.a method: returned the currently-zoomed pane in this
    /// tab. chaoszen has no zoom state; always None.
    pub fn get_zoomed_pane(&self) -> Option<Arc<dyn Pane>> {
        None
    }

    /// Pre-1b.3.a method: focus a pane in the given direction.
    /// chaoszen delegates to zellij; no-op.
    pub fn activate_pane_direction(
        &self,
        _direction: config::keyassignment::PaneDirection,
    ) {
    }

    /// Pre-1b.3.a method: re-balanced split sizes from contained pane
    /// dimensions. chaoszen delegates to zellij; no-op.
    pub fn rebuild_splits_sizes_from_contained_panes(&self) {}

    /// Pre-1b.3.a method: changed a pane's size by a delta.
    /// chaoszen delegates to zellij; no-op.
    pub fn adjust_pane_size(
        &self,
        _direction: config::keyassignment::PaneDirection,
        _amount: usize,
    ) {
    }
}

/// Pre-1b.3.a per-pane snapshot used in IPC sync. chaoszen IPC doesn't
/// carry these; kept as a stub struct so closures supplied to
/// `Tab::sync_with_pane_tree` continue to type-check.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PaneEntry {
    pub window_id: crate::window::WindowId,
    pub tab_id: TabId,
    pub pane_id: crate::pane::PaneId,
    pub title: String,
    pub size: wezterm_term::TerminalSize,
    pub working_dir: Option<SerdeUrl>,
    pub workspace: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Tab>();
    }
}

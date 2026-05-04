//! Cache of rendered pane state, populated by the outbound drain thread.
//!
//! `RenderCache` is the read-side mirror of zellij-server's pane state.
//! The drain thread is the only writer (via `Mux::on_zellij_outbound`).
//! `Pane::get_lines` and friends read from here in Slice 1b.3.b.

use std::collections::HashMap;

use termwiz::surface::Line;

use crate::pane::PaneId;
use crate::renderable::{RenderableDimensions, StableCursorPosition};

#[derive(Default)]
pub struct PaneRenderState {
    pub lines: Vec<Line>,
    pub cursor: StableCursorPosition,
    pub dimensions: RenderableDimensions,
    pub seqno: u64,
    pub title: String,
    pub is_dead: bool,
}

#[derive(Default)]
pub struct RenderCache {
    panes: HashMap<PaneId, PaneRenderState>,
}

impl RenderCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, pane_id: PaneId) -> Option<&PaneRenderState> {
        self.panes.get(&pane_id)
    }

    pub fn upsert(&mut self, pane_id: PaneId, state: PaneRenderState) {
        self.panes.insert(pane_id, state);
    }

    pub fn remove(&mut self, pane_id: PaneId) {
        self.panes.remove(&pane_id);
    }
}

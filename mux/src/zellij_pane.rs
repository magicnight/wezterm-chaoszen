//! Zellij-backed `Pane` implementation. Slice 1b.3.a is the SKELETON:
//! all trait methods present (compile-link works) but bodies are stubs:
//!   - read methods return defaults / placeholder values
//!   - input methods bail with "see Slice 1b.3.c"
//!
//! Slice 1b.3.b fills read methods from `RenderCache`.
//! Slice 1b.3.c fills input methods (forward to zellij-server via
//! ServerHandle) and wires spawn_pane.

use std::ops::Range;
use std::sync::Arc;

use parking_lot::{MappedMutexGuard, Mutex};
use rangeset::RangeSet;
use termwiz::surface::{Line, SequenceNo};
use wezterm_term::color::ColorPalette;
use wezterm_term::{KeyCode, KeyModifiers, MouseEvent, StableRowIndex, Terminal, TerminalSize};

use crate::domain::DomainId;
use crate::pane::{
    CachePolicy, ForEachPaneLogicalLine, LogicalLine, Pane, PaneId, WithPaneLines,
};
use crate::renderable::{
    terminal_get_cursor_position, terminal_get_dimensions, terminal_get_lines,
    RenderableDimensions, StableCursorPosition,
};

pub struct ZellijPane {
    pane_id: PaneId,
    domain_id: DomainId,
    terminal: Mutex<Terminal>,
}

impl ZellijPane {
    pub fn new(pane_id: PaneId, domain_id: DomainId, size: TerminalSize) -> Self {
        let terminal = Terminal::new(
            size,
            Arc::new(config::TermConfig::new()),
            "chaoszen",
            config::wezterm_version(),
            Box::new(std::io::sink()),
        );
        Self {
            pane_id,
            domain_id,
            terminal: Mutex::new(terminal),
        }
    }

    /// Drain thread routes ANSI bytes from `ServerToClientMsg::Render { content }` here.
    ///
    /// Slice 1b.3.b: single-pane assumption — `Mux::on_zellij_outbound` finds
    /// the first ZellijPane in `Mux::panes` and calls this method.
    /// Multi-zellij-pane routing is deferred to 1b.5+.
    pub(crate) fn advance_bytes(&self, bytes: &[u8]) {
        self.terminal.lock().advance_bytes(bytes);
    }
}

#[async_trait::async_trait(?Send)]
impl Pane for ZellijPane {
    fn pane_id(&self) -> PaneId {
        self.pane_id
    }

    fn domain_id(&self) -> DomainId {
        self.domain_id
    }

    fn get_cursor_position(&self) -> StableCursorPosition {
        terminal_get_cursor_position(&mut self.terminal.lock())
    }

    fn get_current_seqno(&self) -> SequenceNo {
        0 // 1b.3.b: read from cache
    }

    fn get_changed_since(
        &self,
        _lines: Range<StableRowIndex>,
        _seqno: SequenceNo,
    ) -> RangeSet<StableRowIndex> {
        RangeSet::default() // 1b.3.b: compare cache seqno
    }

    fn get_lines(&self, lines: Range<StableRowIndex>) -> (StableRowIndex, Vec<Line>) {
        terminal_get_lines(&mut self.terminal.lock(), lines)
    }

    fn with_lines_mut(&self, _lines: Range<StableRowIndex>, _with_lines: &mut dyn WithPaneLines) {
        // 1b.3.b: delegate to get_lines via impl_with_lines_via_get_lines
    }

    fn for_each_logical_line_in_stable_range_mut(
        &self,
        _lines: Range<StableRowIndex>,
        _for_line: &mut dyn ForEachPaneLogicalLine,
    ) {
        // 1b.3.b: iterate logical lines from cache
    }

    fn get_logical_lines(&self, _lines: Range<StableRowIndex>) -> Vec<LogicalLine> {
        vec![] // 1b.3.b: read from cache
    }

    fn get_dimensions(&self) -> RenderableDimensions {
        terminal_get_dimensions(&mut self.terminal.lock())
    }

    fn get_title(&self) -> String {
        self.terminal.lock().get_title().to_string()
    }

    fn is_dead(&self) -> bool {
        false // 1b.3.b: read cache.is_dead
    }

    fn send_paste(&self, _text: &str) -> anyhow::Result<()> {
        anyhow::bail!(
            "ZellijPane::send_paste not implemented in Slice 1b.3.a; see 1b.3.c"
        )
    }

    fn reader(&self) -> anyhow::Result<Option<Box<dyn std::io::Read + Send>>> {
        Ok(None) // 1b.3.c: wire to server output stream
    }

    fn writer(&self) -> MappedMutexGuard<'_, dyn std::io::Write> {
        panic!("ZellijPane::writer not implemented in Slice 1b.3.a; see 1b.3.c")
    }

    fn resize(&self, _size: TerminalSize) -> anyhow::Result<()> {
        anyhow::bail!(
            "ZellijPane::resize not implemented in Slice 1b.3.a; see 1b.3.c"
        )
    }

    fn key_down(&self, _key: KeyCode, _mods: KeyModifiers) -> anyhow::Result<()> {
        anyhow::bail!(
            "ZellijPane::key_down not implemented in Slice 1b.3.a; see 1b.3.c"
        )
    }

    fn key_up(&self, _key: KeyCode, _mods: KeyModifiers) -> anyhow::Result<()> {
        anyhow::bail!(
            "ZellijPane::key_up not implemented in Slice 1b.3.a; see 1b.3.c"
        )
    }

    fn mouse_event(&self, _event: MouseEvent) -> anyhow::Result<()> {
        anyhow::bail!(
            "ZellijPane::mouse_event not implemented in Slice 1b.3.a; see 1b.3.c"
        )
    }

    fn palette(&self) -> ColorPalette {
        ColorPalette::default()
    }

    fn is_mouse_grabbed(&self) -> bool {
        false // 1b.3.b
    }

    fn is_alt_screen_active(&self) -> bool {
        false // 1b.3.b
    }

    fn get_current_working_dir(&self, _policy: CachePolicy) -> Option<url::Url> {
        None // 1b.3.b
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zellij_pane_skeleton_constructs_with_pane_id() {
        let size = wezterm_term::TerminalSize { rows: 24, cols: 80, ..Default::default() };
        let pane = ZellijPane::new(42, 1, size);
        assert_eq!(pane.pane_id(), 42);
        assert_eq!(pane.domain_id(), 1);
    }

    #[test]
    fn zellij_pane_send_paste_returns_unimplemented_in_1b3a() {
        let size = wezterm_term::TerminalSize { rows: 24, cols: 80, ..Default::default() };
        let pane = ZellijPane::new(1, 1, size);
        let err = match pane.send_paste("ls\n") {
            Ok(_) => panic!("expected Err"),
            Err(e) => e,
        };
        assert!(
            err.to_string().contains("1b.3.c"),
            "expected '1b.3.c' in error, got: {}",
            err
        );
    }

    #[test]
    fn advance_bytes_appears_in_get_lines() {
        let size = wezterm_term::TerminalSize {
            rows: 5,
            cols: 20,
            ..Default::default()
        };
        let pane = ZellijPane::new(1, 1, size);
        pane.advance_bytes(b"hello\r\n");

        let (_, lines) = pane.get_lines(0..5);
        assert!(!lines.is_empty(), "expected at least one line");
        let line_text = lines[0].as_str();
        assert!(
            line_text.starts_with("hello"),
            "expected 'hello' prefix, got: {:?}",
            line_text
        );
    }

    #[test]
    fn get_dimensions_matches_initial_size() {
        let size = wezterm_term::TerminalSize {
            rows: 24,
            cols: 80,
            ..Default::default()
        };
        let pane = ZellijPane::new(1, 1, size);
        let dims = pane.get_dimensions();
        assert_eq!(dims.cols, 80);
        assert_eq!(dims.viewport_rows, 24);
    }
}

use crate::RenderPositions;

/// Keeps the state of a [`BinaryDataWidget`](crate::BinaryDataWidget).
#[derive(Debug, Default, Clone, Copy)]
pub struct State {
    pub(super) ensure_selected_in_view_on_next_render: bool,
    pub(super) last_render_positions: Option<RenderPositions>,
    pub(super) offset_address: usize,
    pub(super) selected_address: Option<usize>,
}

impl State {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            ensure_selected_in_view_on_next_render: false,
            last_render_positions: None,
            offset_address: 0,
            selected_address: None,
        }
    }

    #[must_use]
    pub const fn get_offset(&self) -> usize {
        self.offset_address
    }

    #[must_use]
    pub const fn selected(&self) -> Option<usize> {
        self.selected_address
    }

    pub fn select(&mut self, address: Option<usize>) {
        self.selected_address = address;
        self.ensure_selected_in_view_on_next_render = true;

        // TODO: ListState does this. Is this relevant?
        if self.selected_address.is_none() {
            self.offset_address = 0;
        }
    }

    /// Handles the up arrow key.
    pub fn key_up(&mut self) {
        self.selected_address = Some(self.selected_address.map_or(usize::MAX, |selected| {
            let per_row = self
                .last_render_positions
                .map_or(8, |positions| positions.per_row as usize);
            selected.saturating_sub(per_row)
        }));
        self.ensure_selected_in_view_on_next_render = true;
    }

    /// Handles the down arrow key.
    pub fn key_down(&mut self) {
        self.selected_address = Some(self.selected_address.map_or(0, |selected| {
            let per_row = self
                .last_render_positions
                .map_or(8, |positions| positions.per_row as usize);
            selected.saturating_add(per_row)
        }));
        self.ensure_selected_in_view_on_next_render = true;
    }

    /// Handles the left arrow key.
    pub fn key_left(&mut self) {
        self.selected_address = Some(
            self.selected_address
                .map_or(usize::MAX, |selected| selected.saturating_sub(1)),
        );
        self.ensure_selected_in_view_on_next_render = true;
    }

    /// Handles the right arrow key.
    pub fn key_right(&mut self) {
        self.selected_address = Some(
            self.selected_address
                .map_or(0, |selected| selected.saturating_add(1)),
        );
        self.ensure_selected_in_view_on_next_render = true;
    }

    /// Scroll the specified amount of lines up
    pub fn scroll_up(&mut self, lines: usize) {
        let per_row = self
            .last_render_positions
            .map_or(8, |positions| positions.per_row as usize);
        self.offset_address = self
            .offset_address
            .saturating_sub(lines.saturating_mul(per_row));
    }
    /// Scroll the specified amount of lines down
    pub fn scroll_down(&mut self, lines: usize) {
        let per_row = self
            .last_render_positions
            .map_or(8, |positions| positions.per_row as usize);
        self.offset_address = self
            .offset_address
            .saturating_add(lines.saturating_mul(per_row));
    }

    /// Get the address on the given display position of last render
    #[must_use]
    pub fn clicked_address(&self, column: u16, row: u16) -> Option<usize> {
        let address = self
            .last_render_positions?
            .clicked_address(self.offset_address, column, row);
        Some(address)
    }
}

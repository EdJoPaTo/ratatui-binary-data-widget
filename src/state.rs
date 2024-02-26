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
    pub const fn get_offset_address(&self) -> usize {
        self.offset_address
    }

    #[must_use]
    pub const fn selected_address(&self) -> Option<usize> {
        self.selected_address
    }

    pub fn select_address(&mut self, address: Option<usize>) {
        self.selected_address = address;
        self.ensure_selected_in_view_on_next_render = true;
    }

    /// Returns the amount of addresses shown per row on last render
    fn last_per_row(&self) -> usize {
        self.last_render_positions
            .map_or(8, |positions| usize::from(positions.per_row))
    }

    /// Handles the up arrow key.
    pub fn key_up(&mut self) {
        self.select_address(Some(self.selected_address.map_or(usize::MAX, |selected| {
            let per_row = self.last_per_row();
            selected.saturating_sub(per_row)
        })));
    }

    /// Handles the down arrow key.
    pub fn key_down(&mut self) {
        self.select_address(Some(self.selected_address.map_or(0, |selected| {
            let per_row = self.last_per_row();
            selected.saturating_add(per_row)
        })));
    }

    /// Handles the left arrow key.
    pub fn key_left(&mut self) {
        self.select_address(Some(
            self.selected_address
                .map_or(usize::MAX, |selected| selected.saturating_sub(1)),
        ));
    }

    /// Handles the right arrow key.
    pub fn key_right(&mut self) {
        self.select_address(Some(
            self.selected_address
                .map_or(0, |selected| selected.saturating_add(1)),
        ));
    }

    /// Scroll the specified amount of lines up
    pub fn scroll_up(&mut self, lines: usize) {
        self.offset_address = self
            .offset_address
            .saturating_sub(lines.saturating_mul(self.last_per_row()));
    }
    /// Scroll the specified amount of lines down
    pub fn scroll_down(&mut self, lines: usize) {
        self.offset_address = self
            .offset_address
            .saturating_add(lines.saturating_mul(self.last_per_row()));
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

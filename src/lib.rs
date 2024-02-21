#![forbid(unsafe_code)]
#![warn(clippy::pedantic, clippy::nursery)]
#![allow(clippy::cast_possible_truncation)]

/*!
Widget built to show binary data.

The main struct is the [`BinaryDataWidget`].
The user interaction state (like the current selection) is stored in the [`BinaryDataWidgetState`].
*/

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::block::BlockExt;
use ratatui::widgets::{Block, StatefulWidget, Widget};

#[derive(Debug, Clone, Copy)]
struct RenderPositions {
    inner_area: Rect,
    address_width: u16,
    per_row: u16,
    available_data_lines: usize,
    offset_x_hex: u16,
    offset_x_char: u16,
}

impl RenderPositions {
    fn new(inner_area: Rect, data_length: usize) -> Option<Self> {
        const TWO_ADDRESSES_TAKE: u16 = 4 + 2 + 1; // binary + char + whitespace
        const CHAR_OFFSET_PER_TWO: u16 = 4 + 1;

        if inner_area.width < 9 || inner_area.height < 1 || data_length == 0 {
            return None;
        }

        let biggest_address = data_length.saturating_sub(1);
        #[allow(clippy::cast_precision_loss, clippy::cast_sign_loss)]
        let address_width = (biggest_address as f32).log(16.0).ceil() as u16;
        let data_width = inner_area
            .width
            .saturating_sub(2)
            .saturating_sub(address_width);

        let pairs_per_row_max = data_width.saturating_div(TWO_ADDRESSES_TAKE);
        if pairs_per_row_max < 2 {
            return None;
        }

        let pairs_per_row = {
            let mut pairs_per_row: u16 = 1;
            loop {
                let next = pairs_per_row.saturating_mul(2);
                if next > pairs_per_row_max {
                    break;
                }
                pairs_per_row = next;
            }
            pairs_per_row
        };
        let per_row = pairs_per_row.saturating_mul(2);

        let available_data_lines = data_length.div_ceil(per_row as usize);

        let offset_x_hex = inner_area.x.saturating_add(address_width).saturating_add(2);
        let offset_x_char =
            offset_x_hex.saturating_add(pairs_per_row.saturating_mul(CHAR_OFFSET_PER_TWO));

        Some(Self {
            inner_area,
            address_width,
            per_row,
            available_data_lines,
            offset_x_hex,
            offset_x_char,
        })
    }

    const fn x_hex(&self, index_on_row: u16) -> u16 {
        let pair_index = index_on_row.saturating_div(2);
        self.offset_x_hex
            .saturating_add(index_on_row.saturating_mul(2))
            .saturating_add(pair_index)
    }

    const fn x_char(&self, index_on_row: u16) -> u16 {
        self.offset_x_char.saturating_add(index_on_row)
    }

    fn clicked_address(&self, offset_address: usize, column: u16, row: u16) -> usize {
        let row_offset = row.saturating_sub(self.inner_area.top());
        let offset_address = offset_address
            .saturating_add((row_offset as usize).saturating_mul(self.per_row as usize));
        if column <= self.offset_x_hex {
            offset_address
        } else if column < self.offset_x_char.saturating_sub(1) {
            let diff = column.saturating_sub(self.offset_x_hex);
            let index = diff
                .saturating_sub(diff.saturating_div(5))
                .saturating_div(2);
            offset_address.saturating_add(index as usize)
        } else {
            let diff = column.saturating_sub(self.offset_x_char);
            let index = diff.min(self.per_row.saturating_sub(1));
            offset_address.saturating_add(index as usize)
        }
    }
}

/// Keeps the state of a [`BinaryDataWidget`].
#[derive(Debug, Default, Clone, Copy)]
pub struct BinaryDataWidgetState {
    ensure_selected_in_view_on_next_render: bool,
    last_render_positions: Option<RenderPositions>,
    offset_address: usize,
    selected_address: Option<usize>,
}

impl BinaryDataWidgetState {
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

/// A widget to render binary data.
///
//
/// # Example
///
/// ```
/// # use ratatui_binary_data_widget::{BinaryDataWidget, BinaryDataWidgetState};
/// # use ratatui::backend::TestBackend;
/// # use ratatui::Terminal;
/// # use ratatui::widgets::Block;
/// # let mut terminal = Terminal::new(TestBackend::new(32, 32)).unwrap();
/// let mut state = BinaryDataWidgetState::new();
///
/// let data = b"Hello world!";
///
/// terminal.draw(|f| {
///     let area = f.size();
///
///     let widget = BinaryDataWidget::new(data)
///         .block(Block::bordered().title("Binary Data Widget"));
///
///     f.render_stateful_widget(widget, area, &mut state);
/// })?;
/// # Ok::<(), std::io::Error>(())
/// ```
#[derive(Debug, Clone)]
pub struct BinaryDataWidget<'a> {
    data: &'a [u8],

    block: Option<Block<'a>>,
    /// Style used as a base style for the widget
    style: Style,

    /// Style used to render selected item
    highlight_style: Style,
}

impl<'a> BinaryDataWidget<'a> {
    /// Create a new `BinaryDataWidget`.
    #[must_use]
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            block: None,
            style: Style::new(),
            highlight_style: Style::new(),
        }
    }

    #[allow(clippy::missing_const_for_fn)]
    #[must_use]
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    #[must_use]
    pub const fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    #[must_use]
    pub const fn highlight_style(mut self, style: Style) -> Self {
        self.highlight_style = style;
        self
    }

    /// Returns the amount of lines that could be written with the given area width.
    ///
    /// With this information the height of the resulting widget can be limited.
    #[must_use]
    pub fn get_max_lines_of_data_in_area(&self, area: Rect) -> usize {
        let inner = self.block.inner_if_some(area);
        RenderPositions::new(inner, self.data.len())
            .map_or(0, |positions| positions.available_data_lines)
    }
}

impl<'a> StatefulWidget for BinaryDataWidget<'a> {
    type State = BinaryDataWidgetState;

    #[allow(clippy::too_many_lines)]
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        buf.set_style(area, self.style);

        // Get the inner area inside a possible block, otherwise use the full area
        let area = self.block.map_or(area, |block| {
            let inner_area = block.inner(area);
            block.render(area, buf);
            inner_area
        });

        state.last_render_positions = RenderPositions::new(area, self.data.len());
        let Some(positions) = state.last_render_positions else {
            return;
        };
        let RenderPositions {
            address_width,
            per_row,
            available_data_lines,
            ..
        } = positions;

        // Ensure offset is actually in data range
        state.offset_address = state.offset_address.min(self.data.len().saturating_sub(1));
        // Ensure selected_address is actually selectable
        if let Some(selected) = state.selected_address {
            state.selected_address = Some(self.data.len().saturating_sub(1).min(selected));
        }

        let available_height = area.height as usize;

        let mut start_line = state.offset_address.saturating_div(per_row as usize);
        if state.ensure_selected_in_view_on_next_render {
            if let Some(selected_address) = state.selected_address {
                let selected_line = selected_address.saturating_div(per_row as usize);
                if selected_line < start_line {
                    // Move offset up
                    start_line = selected_line;
                } else {
                    let end_line = start_line.saturating_add(available_height);
                    if selected_line >= end_line {
                        // Move offset down
                        let end_line = selected_line.saturating_add(1);
                        start_line = end_line.saturating_sub(available_height);
                    }
                }
            }
            state.offset_address = start_line.saturating_mul(per_row as usize);
            state.ensure_selected_in_view_on_next_render = false;
        }

        let visible_lines = available_data_lines
            .saturating_sub(start_line)
            .min(available_height) as u16;

        let x = area.left();
        let address_width = address_width as usize;

        for line_index in 0..visible_lines {
            const ADDRESS_STYLE: Style = Style::new().fg(Color::Cyan);

            let y = area.top().saturating_add(line_index);

            let offset_address = start_line
                .saturating_add(line_index as usize)
                .saturating_mul(per_row as usize);

            let address_text = format!("{offset_address:>address_width$x}: ");
            buf.set_stringn(x, y, address_text, area.width as usize, ADDRESS_STYLE);

            for i in 0..per_row {
                let address = offset_address.saturating_add(i as usize);
                let Some(value) = self.data.get(address) else {
                    break;
                };
                let char = *value as char;
                let displayable = char.is_ascii_graphic();

                let style = if Some(address) == state.selected_address {
                    self.highlight_style
                } else if *value == 0 {
                    Style::new().fg(Color::DarkGray)
                } else if char.is_ascii_whitespace() {
                    Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else if displayable {
                    Style::new()
                        .fg(Color::LightGreen)
                        .add_modifier(Modifier::BOLD)
                } else if char.is_ascii_control() {
                    Style::new().fg(Color::Red).add_modifier(Modifier::BOLD)
                } else {
                    Style::new()
                };

                // Hex
                {
                    let x = positions.x_hex(i);
                    let text = format!("{value:>2x}");
                    buf.set_string(x, y, text, style);
                }

                // Char
                {
                    let x = positions.x_char(i);
                    if displayable {
                        buf.set_string(x, y, char.to_string(), style);
                    } else {
                        buf.set_string(x, y, "·", style);
                    }
                }
            }
        }
    }
}

impl<'a> Widget for BinaryDataWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut state = BinaryDataWidgetState::new();
        StatefulWidget::render(self, area, buf, &mut state);
    }
}

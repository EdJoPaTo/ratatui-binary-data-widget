#![forbid(unsafe_code)]
#![allow(clippy::cast_possible_truncation)]

/*!
Widget built to show binary data.

The main struct is the [`BinaryDataWidget`].
The user interaction state (like the current selection) is stored in the [`BinaryDataWidgetState`].
*/

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, StatefulWidget, Widget};

/// Keeps the state of a [`BinaryDataWidget`].
#[derive(Debug, Default, Clone, Copy)]
pub struct BinaryDataWidgetState {
    ensure_selected_in_view_on_next_render: bool,
    last_per_row: usize,
    offset_address: usize,
    selected_address: Option<usize>,
}

impl BinaryDataWidgetState {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            ensure_selected_in_view_on_next_render: false,
            last_per_row: 8,
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
            selected.saturating_sub(self.last_per_row)
        }));
        self.ensure_selected_in_view_on_next_render = true;
    }

    /// Handles the down arrow key.
    pub fn key_down(&mut self) {
        self.selected_address = Some(
            self.selected_address
                .map_or(0, |selected| selected.saturating_add(self.last_per_row)),
        );
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
        self.offset_address = self
            .offset_address
            .saturating_sub(lines.saturating_mul(self.last_per_row));
    }
    /// Scroll the specified amount of lines down
    pub fn scroll_down(&mut self, lines: usize) {
        self.offset_address = self
            .offset_address
            .saturating_add(lines.saturating_mul(self.last_per_row));
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
}

impl<'a> StatefulWidget for BinaryDataWidget<'a> {
    type State = BinaryDataWidgetState;

    #[allow(clippy::too_many_lines)]
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        const TWO_ADDRESSES_TAKE: u16 = 4 + 2 + 1; // binary + char + whitespace
        const CHAR_OFFSET_PER_TWO: u16 = 4 + 1;

        buf.set_style(area, self.style);

        // Get the inner area inside a possible block, otherwise use the full area
        let area = self.block.map_or(area, |block| {
            let inner_area = block.inner(area);
            block.render(area, buf);
            inner_area
        });

        if area.width < 1 || area.height < 1 {
            return;
        }

        if self.data.is_empty() {
            return;
        }

        let biggest_address = self.data.len().saturating_sub(1);
        #[allow(clippy::cast_precision_loss, clippy::cast_sign_loss)]
        let address_width = (biggest_address as f32).log(16.0).ceil() as u16;
        let data_width = area.width.saturating_sub(2).saturating_sub(address_width);

        let pairs_per_row_max = data_width.saturating_div(TWO_ADDRESSES_TAKE);
        if pairs_per_row_max < 2 {
            return;
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
        state.last_per_row = per_row as usize;

        // Ensure offset is actually in data range
        state.offset_address = state.offset_address.min(self.data.len().saturating_sub(1));
        // Ensure selected_address is actually selectable
        if let Some(selected) = state.selected_address {
            state.selected_address = Some(self.data.len().saturating_sub(1).min(selected));
        }

        if state.ensure_selected_in_view_on_next_render {
            if let Some(selected_address) = state.selected_address {
                state.offset_address = state
                    .offset_address
                    .min(selected_address.saturating_div(per_row as usize));
            }
        }

        let available_data_lines = self.data.len().div_ceil(per_row as usize);
        let available_height = area.height as usize;

        let mut start_line = state.offset_address.saturating_div(per_row as usize);
        let mut end_line = start_line.saturating_add(available_height);
        if state.ensure_selected_in_view_on_next_render {
            // Move offset down to get selection into view
            if let Some(selected_address) = state.selected_address {
                let selected_line = selected_address.saturating_div(per_row as usize);
                if selected_line >= end_line {
                    end_line = selected_line.saturating_add(1);
                    start_line = end_line.saturating_sub(available_height);
                }
            }
            state.offset_address = start_line.saturating_div(per_row as usize);
        }

        let visible_lines = available_data_lines
            .saturating_sub(start_line)
            .min(available_height) as u16;

        let x = area.left();
        let offset_x_hex = x.saturating_add(address_width).saturating_add(2);
        let offset_x_char =
            offset_x_hex.saturating_add(pairs_per_row.saturating_mul(CHAR_OFFSET_PER_TWO));

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

                let pair_index = i.saturating_div(2);

                // Hex
                {
                    let style = if Some(address) == state.selected_address {
                        self.highlight_style
                    } else if *value > 0 {
                        Style::new()
                    } else {
                        Style::new().fg(Color::DarkGray)
                    };

                    let x = offset_x_hex
                        .saturating_add(i.saturating_mul(2))
                        .saturating_add(pair_index);
                    let text = format!("{value:>2x}");
                    buf.set_string(x, y, text, style);
                }

                // Char
                {
                    let char = *value as char;
                    let x = offset_x_char.saturating_add(i);
                    let displayable = char.is_ascii_graphic();

                    let style = if Some(address) == state.selected_address {
                        self.highlight_style
                    } else if displayable {
                        Style::new()
                    } else {
                        Style::new().fg(Color::DarkGray)
                    };

                    if displayable {
                        buf.set_string(x, y, char.to_string(), style);
                    } else {
                        let non_displayable_style = style.fg(Color::DarkGray);
                        buf.set_string(x, y, "·", non_displayable_style);
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

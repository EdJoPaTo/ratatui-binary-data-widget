#![forbid(unsafe_code)]
#![warn(clippy::pedantic, clippy::nursery)]
#![allow(clippy::cast_possible_truncation)]

/*!
Widget built to show binary data.

The main struct is the [`BinaryDataWidget`].
The user interaction state (like the current selection) is stored in the [`BinaryDataWidgetState`].

For the used colors see the sourcecode of [`color()`].
*/

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::block::BlockExt;
use ratatui::widgets::{
    Block, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget, Widget,
};

mod color;
mod render_positions;
mod state;

pub use color::color;
use render_positions::RenderPositions;
pub use state::State as BinaryDataWidgetState;

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
    fn render(self, full_area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        buf.set_style(full_area, self.style);

        // Get the inner area inside a possible block, otherwise use the full area
        let area = self.block.map_or(full_area, |block| {
            let inner_area = block.inner(full_area);
            block.render(full_area, buf);
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
            .min(available_height);

        {
            // Render Scrollbar
            // When there is a border to the right it is rendered on top.
            // -> Scrollbar and data always visible
            // When there is no border it is still rendered before the binary data
            // -> the scrollbar might not be visible but the data always is
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .track_symbol(None)
                .end_symbol(None);
            let overscroll_workaround = available_data_lines.saturating_sub(available_height);
            let mut scrollbar_state = ScrollbarState::new(overscroll_workaround)
                .position(start_line)
                // Should be available_height but with the current overscroll workaround this looks nicer
                .viewport_content_length(visible_lines);
            let scrollbar_area = Rect {
                // Inner height to be exactly as the content
                y: area.y,
                height: area.height,
                // Outer width to stay on the right border
                x: full_area.x,
                width: full_area.width,
            };
            scrollbar.render(scrollbar_area, buf, &mut scrollbar_state);
        }

        let address_width = address_width as usize;
        let visible_lines = visible_lines as u16;
        let x = area.left();

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
                let character = *value as char;
                let style = if Some(address) == state.selected_address {
                    self.highlight_style
                } else {
                    color::color(character)
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
                    if character.is_ascii_graphic() {
                        buf.set_string(x, y, character.to_string(), style);
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

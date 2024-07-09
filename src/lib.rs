/*!
Widget built to show binary data.

The main struct is the [`BinaryDataWidget`].
The user interaction state (like the current selection) is stored in the [`BinaryDataWidgetState`].

For the used colors see the source code of [`color()`].
*/

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::block::BlockExt;
use ratatui::widgets::{
    Block, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget, Widget,
};

pub use self::color::color;
use self::render_positions::RenderPositions;
pub use self::state::State as BinaryDataWidgetState;

mod color;
mod render_positions;
mod state;

/// A widget to render binary data.
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
///     let block = Block::bordered().title("Binary Data Widget");
///     let widget = BinaryDataWidget::new(data).block(block);
///     f.render_stateful_widget(widget, area, &mut state);
/// })?;
/// # Ok::<(), std::io::Error>(())
/// ```
#[must_use = "The widget is only useful when rendered"]
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
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            block: None,
            style: Style::new(),
            highlight_style: Style::new(),
        }
    }

    #[allow(clippy::missing_const_for_fn)]
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    pub const fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

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
    fn render(self, full_area: Rect, buffer: &mut Buffer, state: &mut Self::State) {
        buffer.set_style(full_area, self.style);

        // Get the inner area inside a possible block, otherwise use the full area
        let area = self.block.map_or(full_area, |block| {
            let inner_area = block.inner(full_area);
            block.render(full_area, buffer);
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
            scrollbar.render(scrollbar_area, buffer, &mut scrollbar_state);
        }

        let address_width = address_width as usize;
        #[allow(clippy::cast_possible_truncation)]
        let visible_lines = visible_lines as u16;
        let x = area.left();

        for line_index in 0..visible_lines {
            const ADDRESS_STYLE: Style = Style::new().fg(Color::Cyan);

            let y = area.top().saturating_add(line_index);

            let offset_address = start_line
                .saturating_add(line_index as usize)
                .saturating_mul(per_row as usize);

            let address_text = format!("{offset_address:>address_width$x}: ");
            buffer.set_stringn(x, y, address_text, area.width as usize, ADDRESS_STYLE);

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
                    buffer.set_string(x, y, text, style);
                }

                // Char
                {
                    let x = positions.x_char(i);
                    let cell = buffer.get_mut(x, y);
                    cell.set_style(style);
                    if character == ' ' {
                        cell.set_symbol(" ");
                    } else if character.is_ascii_graphic() {
                        let array = [*value];
                        let str = unsafe { core::str::from_utf8_unchecked(&array) };
                        cell.set_symbol(str);
                    } else {
                        cell.set_symbol("·");
                    }
                }
            }
        }
    }
}

impl<'a> Widget for BinaryDataWidget<'a> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        let mut state = BinaryDataWidgetState::new();
        StatefulWidget::render(self, area, buffer, &mut state);
    }
}

#[cfg(test)]
mod render_tests {
    use super::*;

    fn render(
        width: u16,
        height: u16,
        data: &[u8],
        mut state: BinaryDataWidgetState,
        expected: &Buffer,
    ) {
        let area = Rect::new(0, 0, width, height);
        let mut buffer = Buffer::empty(area);

        let widget = BinaryDataWidget::new(data);
        StatefulWidget::render(widget, area, &mut buffer, &mut state);

        // Compare without styles
        buffer.set_style(area, Style::reset());
        assert_eq!(&buffer, expected);
    }

    #[test]
    fn numbers() {
        let data: Vec<u8> = (0..=0x12).collect();
        let state = BinaryDataWidgetState::new();
        let expected = Buffer::with_lines([
            " 0:  0 1  2 3 ···· ",
            " 4:  4 5  6 7 ···· ",
            " 8:  8 9  a b ···· ",
            " c:  c d  e f ···· ",
            "10: 1011 12   ···  ",
            "                   ",
        ]);
        render(19, 6, &data, state, &expected);
    }

    #[test]
    fn characters() {
        let data: Vec<u8> = ('A'..='Z').map(|char| char as u8).collect();
        let state = BinaryDataWidgetState::new();
        let expected = Buffer::with_lines([
            " 0: 4142 4344 ABCD ",
            " 4: 4546 4748 EFGH ",
            " 8: 494a 4b4c IJKL ",
            " c: 4d4e 4f50 MNOP ",
            "10: 5152 5354 QRST ",
            "14: 5556 5758 UVWX ",
            "18: 595a      YZ   ",
            "                   ",
        ]);
        render(19, 8, &data, state, &expected);
    }
}

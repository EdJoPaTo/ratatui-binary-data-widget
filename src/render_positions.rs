use ratatui::layout::Rect;

#[derive(Debug, Clone, Copy)]
pub struct RenderPositions {
    pub inner_area: Rect,
    pub biggest_address: usize,
    pub address_width: u16,
    pub per_row: u16,
    pub available_data_lines: usize,
    pub offset_x_hex: u16,
    pub offset_x_char: u16,
}

impl RenderPositions {
    pub fn new(inner_area: Rect, data_length: usize) -> Option<Self> {
        const TWO_ADDRESSES_TAKE: u16 = 4 + 2 + 1; // binary + char + whitespace
        const CHAR_OFFSET_PER_TWO: u16 = 4 + 1;

        if inner_area.width < 9 || inner_area.height < 1 || data_length == 0 {
            return None;
        }

        let biggest_address = data_length.saturating_sub(1);
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_precision_loss,
            clippy::cast_sign_loss
        )]
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
            biggest_address,
            address_width,
            per_row,
            available_data_lines,
            offset_x_hex,
            offset_x_char,
        })
    }

    pub const fn x_hex(&self, index_on_row: u16) -> u16 {
        let pair_index = index_on_row.saturating_div(2);
        self.offset_x_hex
            .saturating_add(index_on_row.saturating_mul(2))
            .saturating_add(pair_index)
    }

    pub const fn x_char(&self, index_on_row: u16) -> u16 {
        self.offset_x_char.saturating_add(index_on_row)
    }

    pub fn clicked_address(&self, offset_address: usize, column: u16, row: u16) -> usize {
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

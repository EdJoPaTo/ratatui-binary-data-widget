use ratatui::style::{Color, Modifier, Style};

/// Returns a [`Style`] which is used to style the given `character` on render.
#[must_use]
pub const fn color(character: char) -> Style {
    if character as u8 == 0 {
        Style::new().fg(Color::DarkGray)
    } else if character as u8 == 0xff {
        Style::new().fg(Color::Blue).add_modifier(Modifier::BOLD)
    } else if character.is_ascii_whitespace() {
        Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else if character.is_ascii_graphic() {
        Style::new()
            .fg(Color::LightGreen)
            .add_modifier(Modifier::BOLD)
    } else if character.is_ascii_control() {
        Style::new().fg(Color::Red).add_modifier(Modifier::BOLD)
    } else {
        Style::new()
    }
}

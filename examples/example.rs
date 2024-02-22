use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Span,
    widgets::Block,
    Terminal,
};
use std::error::Error;
use std::fs;
use std::path::Path;

use ratatui_binary_data_widget::{BinaryDataWidget, BinaryDataWidgetState};

struct App<'a> {
    data: &'a [u8],
    state: BinaryDataWidgetState,
}

impl<'a> App<'a> {
    const fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            state: BinaryDataWidgetState::new(),
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("Usage: optional argument - path to file to be read and displayed");
    println!("Otherwise the executable itself will be read.");
    let path = std::env::args_os()
        .last()
        .expect("The executable itself should always be an argument");
    let path = Path::new(&path);
    println!("Read file: {path:?}");
    let data = fs::read(path).expect("should be able to read the file");
    println!("Success. Show terminal ui.");

    // Terminal initialization
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // App
    let app = App::new(&data);
    let res = run_app(&mut terminal, app);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> std::io::Result<()> {
    let mut update = true;
    loop {
        if update {
            terminal.draw(|frame| {
                let area = frame.size();
                let widget = BinaryDataWidget::new(app.data)
                    .block(Block::bordered().title("Binary Data Widget"))
                    .highlight_style(
                        Style::new()
                            .fg(Color::Black)
                            .bg(Color::LightGreen)
                            .add_modifier(Modifier::BOLD),
                    );
                frame.render_stateful_widget(widget, area, &mut app.state);

                if let Some(selected) = app.state.selected() {
                    let meta = format!("Selected: {selected:x}");
                    let meta_area = Rect::new(1, area.height - 1, area.width - 1, 1);
                    frame.render_widget(Span::raw(meta), meta_area);
                }
            })?;
        }
        let area = terminal.size().expect("Should have a size");
        update = match handle_events(&mut app, area)? {
            Update::Quit => return Ok(()),
            Update::Redraw => true,
            Update::Skip => false,
        };
    }
}

enum Update {
    Quit,
    Redraw,
    Skip,
}

/// Returns true when the widget should be updated
fn handle_events(app: &mut App, area: Rect) -> std::io::Result<Update> {
    match event::read()? {
        Event::Key(key) => match key.code {
            KeyCode::Char('q') => return Ok(Update::Quit),
            KeyCode::Esc => app.state.select(None),
            KeyCode::Home => app.state.select(Some(0)),
            KeyCode::End => app.state.select(Some(usize::MAX)),
            KeyCode::Left => app.state.key_left(),
            KeyCode::Right => app.state.key_right(),
            KeyCode::Down => app.state.key_down(),
            KeyCode::Up => app.state.key_up(),
            KeyCode::PageDown => app.state.scroll_down((area.height / 2) as usize),
            KeyCode::PageUp => app.state.scroll_up((area.height / 2) as usize),
            _ => return Ok(Update::Skip),
        },
        Event::Mouse(event) => match event.kind {
            event::MouseEventKind::ScrollDown => app.state.scroll_down(1),
            event::MouseEventKind::ScrollUp => app.state.scroll_up(1),
            event::MouseEventKind::Down(_) => {
                if let Some(address) = app.state.clicked_address(event.column, event.row) {
                    app.state.select(Some(address));
                }
            }
            _ => return Ok(Update::Skip),
        },
        Event::Resize(_, _) => return Ok(Update::Redraw),
        _ => return Ok(Update::Skip),
    }
    Ok(Update::Redraw)
}

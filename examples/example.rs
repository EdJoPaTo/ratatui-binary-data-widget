use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    style::{Color, Modifier, Style},
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
    loop {
        terminal.draw(|f| {
            let area = f.size();

            let items = BinaryDataWidget::new(app.data)
                .block(Block::bordered().title("Binary Data Widget"))
                .highlight_style(
                    Style::new()
                        .fg(Color::Black)
                        .bg(Color::LightGreen)
                        .add_modifier(Modifier::BOLD),
                );
            f.render_stateful_widget(items, area, &mut app.state);
        })?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Left => app.state.key_left(),
                KeyCode::Right => app.state.key_right(),
                KeyCode::Down => app.state.key_down(),
                KeyCode::Up => app.state.key_up(),
                _ => {}
            }
        }
    }
}

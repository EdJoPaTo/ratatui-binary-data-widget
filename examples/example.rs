use std::error::Error;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use crossterm::event::{Event, KeyCode, MouseEventKind};
use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::{Frame, Terminal};

use ratatui_binary_data_widget::{BinaryDataWidget, BinaryDataWidgetState};

enum Update {
    Quit,
    Redraw,
    Skip,
}

struct App<'a> {
    data: &'a [u8],
    last_area: Rect,
    state: BinaryDataWidgetState,
}

impl<'a> App<'a> {
    const fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            last_area: Rect {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            },
            state: BinaryDataWidgetState::new(),
        }
    }

    fn on_event(&mut self, event: &Event) -> Update {
        match event {
            Event::Key(key) => match key.code {
                KeyCode::Char('q') => return Update::Quit,
                KeyCode::Esc => self.state.select_address(None),
                KeyCode::Home => self.state.select_address(Some(0)),
                KeyCode::End => self.state.select_address(Some(usize::MAX)),
                KeyCode::Left => self.state.key_left(),
                KeyCode::Right => self.state.key_right(),
                KeyCode::Down => self.state.key_down(),
                KeyCode::Up => self.state.key_up(),
                KeyCode::PageDown => self.state.scroll_down((self.last_area.height / 2) as usize),
                KeyCode::PageUp => self.state.scroll_up((self.last_area.height / 2) as usize),
                _ => return Update::Skip,
            },
            Event::Mouse(event) => match event.kind {
                MouseEventKind::ScrollDown => self.state.scroll_down(1),
                MouseEventKind::ScrollUp => self.state.scroll_up(1),
                MouseEventKind::Down(_) => {
                    if let Some(address) = self.state.clicked_address(event.column, event.row) {
                        self.state.select_address(Some(address));
                    }
                }
                _ => return Update::Skip,
            },
            Event::Resize(_, _) => return Update::Redraw,
            _ => return Update::Skip,
        }
        Update::Redraw
    }

    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.size();
        self.last_area = area;
        let widget = BinaryDataWidget::new(self.data)
            .block(Block::bordered().title("Binary Data Widget"))
            .highlight_style(
                Style::new()
                    .fg(Color::Black)
                    .bg(Color::LightGreen)
                    .add_modifier(Modifier::BOLD),
            );
        frame.render_stateful_widget(widget, area, &mut self.state);

        if let Some(selected) = self.state.selected_address() {
            let meta = format!("Selected: {selected:x}");
            let meta_area = Rect::new(1, area.height - 1, area.width - 1, 1);
            frame.render_widget(Span::raw(meta), meta_area);
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
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    crossterm::execute!(
        stdout,
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    // App
    let app = App::new(&data);
    let res = run_app(&mut terminal, app);

    // restore terminal
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> std::io::Result<()> {
    const DEBOUNCE: Duration = Duration::from_millis(20); // 50 FPS
    terminal.draw(|frame| app.draw(frame))?;
    let mut debounce: Option<Instant> = None;
    loop {
        let timeout = debounce.map_or(DEBOUNCE, |since| DEBOUNCE.saturating_sub(since.elapsed()));
        if crossterm::event::poll(timeout)? {
            match app.on_event(&crossterm::event::read()?) {
                Update::Quit => return Ok(()),
                Update::Redraw => {
                    debounce.get_or_insert_with(Instant::now);
                }
                Update::Skip => {}
            }
        }
        if debounce.is_some_and(|since| since.elapsed() > DEBOUNCE) {
            terminal.draw(|frame| app.draw(frame))?;
            debounce = None;
        }
    }
}

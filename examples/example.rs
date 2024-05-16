use std::error::Error;
use std::fmt::Write;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use crossterm::event::{Event, KeyCode, KeyModifiers, MouseEventKind};
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
    render_times: Vec<Duration>,
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
            render_times: Vec::new(),
            state: BinaryDataWidgetState::new(),
        }
    }

    fn on_event(&mut self, event: &Event) -> Update {
        let change = match event {
            Event::Key(key) => match key.code {
                KeyCode::Char('q') => return Update::Quit,
                KeyCode::Esc => self.state.select_address(None),
                KeyCode::Home if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.state.select_address(Some(0))
                }
                KeyCode::End if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.state.select_address(Some(usize::MAX))
                }
                KeyCode::Home => self.state.select_first_in_row(),
                KeyCode::End => self.state.select_last_in_row(),
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
                MouseEventKind::Down(_) => self.state.select_at(event.column, event.row),
                _ => return Update::Skip,
            },
            Event::Resize(_, _) => return Update::Redraw,
            _ => return Update::Skip,
        };
        if change {
            Update::Redraw
        } else {
            Update::Skip
        }
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
        let instant = Instant::now();
        frame.render_stateful_widget(widget, area, &mut self.state);
        self.render_times.push(instant.elapsed());

        #[allow(clippy::cast_precision_loss)]
        let average_render_time = self
            .render_times
            .iter()
            .sum::<Duration>()
            .div_f64(self.render_times.len() as f64);
        let mut meta = format!("Avg render time: {average_render_time:?}");
        if let Some(selected) = self.state.selected_address() {
            _ = write!(meta, " Selected: {selected:x}");
        }
        let meta_area = Rect::new(1, area.height - 1, area.width - 1, 1);
        frame.render_widget(Span::raw(meta), meta_area);
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
    const INTERVAL: Duration = Duration::from_millis(200); // 5 FPS
    const DEBOUNCE: Duration = Duration::from_millis(20); // 50 FPS
    terminal.draw(|frame| app.draw(frame))?;
    let mut debounce: Option<Instant> = None;
    let mut last_render = Instant::now();
    loop {
        let timeout = debounce.map_or(INTERVAL, |since| DEBOUNCE.saturating_sub(since.elapsed()));
        if crossterm::event::poll(timeout)? {
            match app.on_event(&crossterm::event::read()?) {
                Update::Quit => return Ok(()),
                Update::Redraw => {
                    debounce.get_or_insert_with(Instant::now);
                }
                Update::Skip => {}
            }
        }
        if debounce.map_or_else(
            || last_render.elapsed() > INTERVAL,
            |since| since.elapsed() > DEBOUNCE,
        ) {
            terminal.draw(|frame| app.draw(frame))?;
            debounce = None;
            last_render = Instant::now();
        }
    }
}

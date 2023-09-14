mod app;
mod ui;
use app::{App, Mode};
use ratatui::{
    prelude::{Backend, CrosstermBackend},
    Terminal,
};
use ui::ui;

use std::{
    error::Error,
    time::{Duration, Instant},
};

use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    tick_rate: Duration,
) -> std::io::Result<()> {
    let mut last_tick = Instant::now();

    let mut input = None;

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match (&app.mode, key.code) {
                        (Mode::Normal | Mode::Visual, KeyCode::Char('q')) => return Ok(()),
                        (Mode::Normal | Mode::Visual, KeyCode::Char('h')) => app.left(),
                        (Mode::Normal | Mode::Visual, KeyCode::Char('j')) => app.down(),
                        (Mode::Normal | Mode::Visual, KeyCode::Char('k')) => app.up(),
                        (Mode::Normal | Mode::Visual, KeyCode::Char('l')) => app.right(),
                        (Mode::Normal | Mode::Visual, KeyCode::Char('i')) => {
                            app.mode = Mode::Insert
                        }
                        (Mode::Normal, KeyCode::Char('w')) => app.flush(),
                        (Mode::Normal | Mode::Visual, KeyCode::Char('d')) => {
                            app.delete();
                            app.selection.set(app.selection.start);
                            app.mode = Mode::Normal;
                            app.left();
                        }
                        (Mode::Normal, KeyCode::Char('o')) => {
                            app.append();
                            app.mode = Mode::Insert;
                            app.right();
                        }
                        (Mode::Normal, KeyCode::Char('v')) => app.mode = Mode::Visual,
                        (Mode::Normal | Mode::Visual, KeyCode::Char('H')) => {
                            app.highlight();
                            app.mode = Mode::Normal;
                        }
                        (Mode::Normal, KeyCode::Char('g')) => app.selection.set(0),
                        (Mode::Normal, KeyCode::Char('0')) => app
                            .selection
                            .set(app.selection.start - app.selection.start % 16),
                        (_, KeyCode::Esc) => app.mode = Mode::Normal,
                        (Mode::Insert, KeyCode::Char(c)) => match (input, c.to_digit(16)) {
                            (None, Some(b)) => input = Some(b),
                            (Some(a), Some(b)) => {
                                app.set((a * 16 + b) as u8);
                                app.right();
                                input = None;
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}

#[derive(Parser, Debug)]
struct Args {
    file: Option<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let mut data = match &args.file {
        Some(f) => std::fs::read(f).unwrap_or(vec![0]),
        None => vec![0],
    };

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let tick_rate = Duration::from_millis(250);

    let app = App::new(&mut data, args.file.as_deref());
    let res = run_app(&mut terminal, app, tick_rate);

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

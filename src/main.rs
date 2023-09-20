mod comparator;
mod ui;
mod viewer;

use comparator::Comparator;
use ratatui::{
    prelude::{Backend, CrosstermBackend},
    Terminal,
};
use ui::{comparator_ui, viewer_ui};
use viewer::{Mode, Viewer};

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

fn run_viewer<B: Backend>(
    terminal: &mut Terminal<B>,
    mut viewer: Viewer,
    tick_rate: Duration,
) -> std::io::Result<()> {
    let mut last_tick = Instant::now();

    let mut input = None;

    loop {
        terminal.draw(|f| viewer_ui(f, &mut viewer))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match (&viewer.mode, key.code) {
                        (Mode::Normal | Mode::Visual, KeyCode::Char('q')) => return Ok(()),
                        (Mode::Normal | Mode::Visual, KeyCode::Char('h')) => viewer.left(),
                        (Mode::Normal | Mode::Visual, KeyCode::Char('j')) => viewer.down(),
                        (Mode::Normal | Mode::Visual, KeyCode::Char('k')) => viewer.up(),
                        (Mode::Normal | Mode::Visual, KeyCode::Char('l')) => viewer.right(),
                        (Mode::Normal | Mode::Visual, KeyCode::Char('i')) => {
                            viewer.mode = Mode::Insert
                        }
                        (Mode::Normal, KeyCode::Char('w')) => viewer.flush(),
                        (Mode::Normal | Mode::Visual, KeyCode::Char('d')) => {
                            viewer.delete();
                            viewer.selection.set(viewer.selection.start);
                            viewer.mode = Mode::Normal;
                            viewer.left();
                        }
                        (Mode::Normal, KeyCode::Char('o')) => {
                            viewer.append();
                            viewer.mode = Mode::Insert;
                            viewer.right();
                        }
                        (Mode::Normal, KeyCode::Char('v')) => viewer.mode = Mode::Visual,
                        (Mode::Normal | Mode::Visual, KeyCode::Char('H')) => {
                            viewer.highlight();
                            viewer.mode = Mode::Normal;
                        }
                        (Mode::Normal, KeyCode::Char('g')) => viewer.selection.set(0),
                        (Mode::Normal, KeyCode::Char('G')) => {
                            viewer.selection.set(viewer.data.len() - 1)
                        }
                        (Mode::Normal, KeyCode::Char('0')) => viewer
                            .selection
                            .set(viewer.selection.start - viewer.selection.start % 16),
                        (_, KeyCode::Esc) => viewer.mode = Mode::Normal,
                        (Mode::Insert, KeyCode::Char(c)) => match (input, c.to_digit(16)) {
                            (None, Some(b)) => input = Some(b),
                            (Some(a), Some(b)) => {
                                viewer.set(Some((a * 16 + b) as u8));
                                viewer.right();
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

fn run_comparator<B: Backend>(
    terminal: &mut Terminal<B>,
    mut comparator: Comparator,
    tick_rate: Duration,
) -> std::io::Result<()> {
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| comparator_ui(f, &mut comparator))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Char('h') => comparator.left(),
                        KeyCode::Char('j') => comparator.down(),
                        KeyCode::Char('k') => comparator.up(),
                        KeyCode::Char('l') => comparator.right(),
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
    other: Option<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let tick_rate = Duration::from_millis(250);

    let res = match (&args.file, &args.other) {
        (None, None) => {
            let mut data = vec![Some(0)];
            let viewer = Viewer::new(&mut data, args.file.as_deref());
            run_viewer(&mut terminal, viewer, tick_rate)
        }
        (Some(f), None) => {
            let mut data = std::fs::read(f)
                .unwrap_or(vec![0])
                .into_iter()
                .map(|n| Some(n))
                .collect();
            let viewer = Viewer::new(&mut data, args.file.as_deref());
            run_viewer(&mut terminal, viewer, tick_rate)
        }
        (Some(a), Some(b)) => {
            let mut adata = std::fs::read(a)
                .unwrap_or(vec![0])
                .into_iter()
                .map(|n| Some(n))
                .collect();
            let mut bdata = std::fs::read(b)
                .unwrap_or(vec![0])
                .into_iter()
                .map(|n| Some(n))
                .collect();
            let comparator = Comparator::new(&mut adata, &mut bdata, a, b);
            run_comparator(&mut terminal, comparator, tick_rate)
        }
        (None, Some(_)) => unreachable!(),
    };

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

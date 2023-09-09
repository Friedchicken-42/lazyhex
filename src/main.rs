use std::{
    error::Error,
    io,
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{layout::Constraint::*, prelude::*, widgets::*};

struct App<'a> {
    data: &'a [u8],
    selected: usize,
}

impl<'a> App<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { selected: 0, data }
    }

    fn left(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    fn right(&mut self) {
        if self.selected < self.data.len() - 1 {
            self.selected += 1;
        }
    }

    fn up(&mut self) {
        if self.selected < 16 {
            self.selected = 0;
        } else {
            self.selected -= 16;
        }
    }

    fn down(&mut self) {
        self.selected = std::cmp::min(self.data.len() - 1, self.selected + 16);
    }
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    tick_rate: Duration,
) -> io::Result<()> {
    let mut last_tick = Instant::now();
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Char('h') => app.left(),
                        KeyCode::Char('j') => app.down(),
                        KeyCode::Char('k') => app.up(),
                        KeyCode::Char('l') => app.right(),
                        // KeyCode::Down => app.list.next(),
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

fn slice(data: &[u8], offset: usize, length: usize) -> Vec<u8> {
    let mut v = vec![0; length];

    for i in 0..length {
        v[i] = match data.get(i + offset) {
            Some(n) => *n,
            None => 0,
        };
    }

    v
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let rects = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![Length(82), Min(0)])
        .split(f.size());

    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![Length(11), Length(52), Length(19)])
        .split(rects[0]);

    let mut spans: Vec<_> = app
        .data
        .chunks(16)
        .map(|chunk| {
            chunk
                .iter()
                .map(|n| Span::from(format!("{n:02x}")))
                .collect::<Vec<_>>()
        })
        .collect();

    let indexes: Vec<_> = (0..spans.len())
        .map(|i| Line::from(format!("0x{i:05X}0")))
        .collect();

    spans[app.selected / 16][app.selected % 16].patch_style(Style::default().bg(Color::DarkGray));

    let spans = spans
        .into_iter()
        .map(|chunk| {
            let len = chunk.len();
            let chunk = chunk.into_iter().chain((len..16).map(|_| Span::raw("  ")));
            let mut chunk: Vec<_> = chunk.flat_map(|span| [span, Span::raw(" ")]).collect();
            chunk.insert(15, Span::raw(" "));

            chunk
        })
        .map(|chunk| Line::from(chunk));

    let mut header: Vec<_> = (0..16).map(|i| Span::from(format!(" {i:x} "))).collect();
    header.insert(8, Span::raw(" "));
    let header = Line::from(header);
    let spans: Vec<_> = [header].into_iter().chain(spans).collect();

    let table: Vec<_> = app
        .data
        .chunks(16)
        .map(|chunk| {
            (0..16)
                .map(|i| match chunk.get(i) {
                    Some(&c) if c > 32 && c != 127 => c as char,
                    Some(_) => '.',
                    None => ' ',
                })
                .collect::<String>()
        })
        .map(|s| Line::from(s))
        .collect();

    let paragraph = Paragraph::new(indexes)
        .block(Block::default().padding(Padding {
            left: 1,
            right: 1,
            top: 2,
            bottom: 1,
        }))
        .alignment(Alignment::Right);
    let p1 = Paragraph::new(spans)
        .block(
            Block::default()
                .padding(Padding::uniform(1))
                .borders(Borders::RIGHT | Borders::LEFT),
        )
        .alignment(Alignment::Center);
    let p2 = Paragraph::new(table)
        .block(Block::default().padding(Padding {
            left: 1,
            right: 1,
            top: 2,
            bottom: 1,
        }))
        .alignment(Alignment::Left);

    f.render_widget(paragraph, main[0]);
    f.render_widget(p1, main[1]);
    f.render_widget(p2, main[2]);

    let block = Block::default().borders(Borders::ALL).title("Hex");
    f.render_widget(block, rects[0]);

    let byte = slice(&app.data, app.selected, 1)[0];

    let mut integer: u32 = 0;
    for b in slice(&app.data, app.selected, 4) {
        integer = (integer << 8) | u32::from(b);
    }

    let items = [
        ListItem::new(format!("hex:   0x{:02x}", byte)),
        ListItem::new(format!("uint8: {}", byte)),
        ListItem::new(format!("char:  {}", byte as char)),
        ListItem::new(format!("int:   {}", integer)),
    ];
    let list = List::new(items).block(
        Block::default()
            .title("Info")
            .borders(Borders::ALL)
            .padding(Padding::uniform(1)),
    );
    f.render_widget(list, rects[1])
}

fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let tick_rate = Duration::from_millis(250);

    let data = b"0123456789abcdef0123456789ghijkl0".to_vec();
    let app = App::new(&data);
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

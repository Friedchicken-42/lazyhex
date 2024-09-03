mod app;
mod config;

use num_traits::ops::bytes::FromBytes;
use std::{
    io::{stdout, Stdout},
    ops::Range,
};

use app::{App, Highlight, Mode, Selection};
use clap::Parser;
use config::Endian;
use mlua::Lua;
use ratatui::{
    backend::{Backend, CrosstermBackend},
    buffer::Buffer,
    crossterm::{
        event::{self, Event, KeyCode, KeyModifiers},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Padding, Widget},
    Frame, Terminal,
};

use anyhow::Result;

struct TerminalManager {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalManager {
    fn new() -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }
}

impl Drop for TerminalManager {
    fn drop(&mut self) {
        disable_raw_mode().unwrap();
        execute!(self.terminal.backend_mut(), LeaveAlternateScreen).unwrap();
        self.terminal.show_cursor().unwrap();
    }
}

fn get_style(highlights: &[Highlight], area: Range<usize>, selection: &Selection) -> Style {
    let mut style = Style::default();

    for highlight in highlights {
        if area.start >= highlight.start && area.end < highlight.end {
            if let Some(bg) = highlight.bg {
                style = style.bg(bg);
            }
            if let Some(fg) = highlight.fg {
                style = style.fg(fg);
            }
        }
    }

    match selection {
        Selection::Single(current) if area.start == *current && area.end == *current => {
            Style::default().bg(Color::White).fg(Color::Black)
        }
        Selection::Visual { current, .. } if area.start == *current && area.end == *current => {
            Style::default().bg(Color::White).fg(Color::Black)
        }
        Selection::Visual { range, .. } if range.start <= area.start && range.end > area.end => {
            Style::default().bg(Color::Gray).fg(Color::Black)
        }
        _ => style,
    }
}

struct IndexView<'a> {
    app: &'a App<'a>,
}

impl<'a> Widget for IndexView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let current = self.app.single_selection() / 16;

        let range = self.app.visible_range();
        let start = range.start / 16;
        let end = (start + self.app.height as usize).min(range.end / 16 + 1);

        for (i, index) in (start..end).enumerate() {
            let hex = format!("0x{index:05X}0");
            let style = if current == index {
                Style::default().bg(Color::White).fg(Color::Black)
            } else {
                Style::default()
            };
            buf.set_string(area.x, area.y + i as u16 + 1, hex, style);
        }
    }
}

struct HexView<'a> {
    app: &'a App<'a>,
}

impl<'a> Widget for HexView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let range = self.app.visible_range();
        let offset = range.start;
        let data = &self.app.data[range];
        let selection = &self.app.selection;

        for (y, chunk) in data.chunks(16).enumerate() {
            let mut x = 2;

            for (i, byte) in chunk.iter().enumerate() {
                let pos = offset + y * 16 + i;
                let y = y as u16 + 1;

                if i == 8 {
                    buf.set_string(area.x + x, area.y + y, "  ", Style::default());
                    x += 2;
                } else if i != 0 {
                    let style = get_style(&self.app.highlights, (pos - 1)..pos, selection);
                    buf.set_string(area.x + x, area.y + y, " ", style);
                    x += 1;
                }

                let hex = format!("{byte:02x}");
                let style = get_style(&self.app.highlights, pos..pos, selection);

                buf.set_string(area.x + x, area.y + y, hex, style);

                x += 2;
            }
        }
    }
}

struct TextView<'a> {
    app: &'a App<'a>,
}

impl<'a> Widget for TextView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let range = self.app.visible_range();
        let offset = range.start;
        let data = &self.app.data[range];

        for (y, chunk) in data.chunks(16).enumerate() {
            for (x, byte) in chunk.iter().enumerate() {
                let pos = offset + y * 16 + x;
                let y = y as u16 + 1;

                let string = match byte {
                    &c if c > 32 && c < 127 => format!("{}", c as char),
                    _ => String::from("."),
                };

                let style = get_style(&self.app.highlights, pos..pos, &self.app.selection);
                buf.set_string(area.x + x as u16, area.y + y, string, style);
            }
        }
    }
}

struct InfoView<'a> {
    app: &'a App<'a>,
}

impl<'a> Widget for InfoView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let data = &self.app.data;
        let config = &self.app.config;

        let current = self.app.single_selection();

        let byte = data[current];

        let (big, little) = match config.endian {
            Endian::Big => (
                Style::default().bg(Color::White).fg(Color::Black),
                Style::default().fg(Color::White),
            ),
            Endian::Little => (
                Style::default().fg(Color::White),
                Style::default().bg(Color::White).fg(Color::Black),
            ),
        };

        buf.set_string(area.x + 2, area.y, " Big ", big);
        buf.set_string(area.x + 7, area.y, " Little ", little);

        // TODO: find a way to remove `N` and use `std::mem::size_of::<T>()`
        fn read<T: FromBytes<Bytes = [u8; N]>, const N: usize>(data: &[u8], endian: Endian) -> T {
            let mut bytes = [0u8; N];
            let length = N.min(data.len());
            bytes[..length].copy_from_slice(&data[..length]);

            match endian {
                Endian::Little => T::from_le_bytes(&bytes),
                Endian::Big => T::from_be_bytes(&bytes),
            }
        }

        let short = read::<u16, 2>(&data[current..], config.endian);
        let int = read::<u32, 4>(&data[current..], config.endian);
        let long = read::<u64, 8>(&data[current..], config.endian);
        let float = read::<f32, 4>(&data[current..], config.endian);
        let double = read::<f64, 8>(&data[current..], config.endian);

        let mut string = match &self.app.selection {
            Selection::Single(_) => String::from(byte as char),
            Selection::Visual { range, .. } => match std::str::from_utf8(&data[range.clone()]) {
                Ok(s) => s.to_string(),
                Err(_) => String::from("* not utf8 *"),
            },
        };
        string.truncate(20);

        let info = [
            ("hex", format!("0x{byte:02x}")),
            ("binary", format!("0b{byte:08b}")),
            ("octal", format!("0o{byte:o}")),
            ("u8", format!("{}", byte)),
            ("i8", format!("{}", byte as i8)),
            ("char", format!("{:?}", byte as char)),
            ("u16", format!("{}", short)),
            ("i16", format!("{}", short as i16)),
            ("u32", format!("{}", int)),
            ("i32", format!("{}", int as i32)),
            ("u64", format!("{}", long)),
            ("i64", format!("{}", long as i64)),
            ("f32", format!("{:.5e}", float)),
            ("f64", format!("{:.5e}", double)),
            ("string", format!("{string:?}")),
        ];

        let mut y = 2;
        for (name, value) in info {
            let style = Style::default();
            buf.set_string(area.x, area.y + y, format!("{name}:"), style);
            buf.set_string(area.x + 8, area.y + y, value, style);
            y += 1;
        }
    }
}

struct HighlightsView<'a> {
    app: &'a App<'a>,
}

impl<'a> Widget for HighlightsView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let current = self.app.single_selection();
        let mut y = 0;

        for highlight in &self.app.highlights {
            if current >= highlight.start && current < highlight.end {
                let mut style = Style::default();
                if let Some(bg) = highlight.bg {
                    style = style.bg(bg);
                }
                if let Some(fg) = highlight.fg {
                    style = style.fg(fg);
                }
                buf.set_string(area.x, area.y + y, &highlight.text, style);

                y += 1;
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let width = f.area().width;
    let height = f.area().height;

    const INDEX: u16 = 9;
    const HEX: u16 = 53;
    const TEXT: u16 = 16;
    const MAIN: u16 = INDEX + HEX + TEXT + 4;
    const RIGHT: u16 = 38;

    let constraints = if width > MAIN + RIGHT {
        vec![Constraint::Length(MAIN), Constraint::Length(RIGHT)]
    } else {
        vec![Constraint::Length(MAIN)]
    };

    let main = Layout::horizontal(constraints)
        .flex(Flex::Center)
        .split(f.area());

    if width > MAIN + RIGHT {
        let right = if height > 20 + 20 {
            vec![Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]
        } else {
            vec![Constraint::Ratio(1, 1)]
        };

        let rights = Layout::vertical(right).split(main[1]);

        let info = InfoView { app };

        let info_block = Block::default()
            .borders(Borders::ALL)
            .padding(Padding::uniform(1))
            .title(" Info ");
        let inner_info_block = info_block.inner(rights[0]);

        f.render_widget(info_block, rights[0]);
        f.render_widget(info, inner_info_block);

        if height > 20 + 20 {
            let highlights = HighlightsView { app };
            let high_block = Block::default()
                .borders(Borders::ALL)
                .padding(Padding::uniform(1))
                .title(" Highlights ");
            let high_info_block = high_block.inner(rights[1]);

            f.render_widget(high_block, rights[1]);
            f.render_widget(highlights, high_info_block);
        }
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .padding(Padding::new(1, 1, 0, 0))
        .title(" Hex ");

    let inner = block.inner(main[0]);

    let rects = Layout::horizontal([
        Constraint::Length(INDEX),
        Constraint::Length(HEX),
        Constraint::Length(TEXT),
    ])
    .split(inner);

    app.height = inner.height - 1;

    let indexview = IndexView { app };
    let hexview = HexView { app };
    let textview = TextView { app };

    let header = Line::from("   0  1  2  3  4  5  6  7   8  9  a  b  c  d  e  f");
    f.render_widget(header, rects[1]);

    f.render_widget(indexview, rects[0]);
    f.render_widget(hexview, rects[1]);
    f.render_widget(textview, rects[2]);

    f.render_widget(block, main[0]);
}

fn run_draw_loop<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> Result<()> {
    let mut prev = None;

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        #[allow(clippy::single_match)]
        match event::read()? {
            Event::Key(key) => match (app.mode, key.code) {
                (_, KeyCode::Char('q')) => return Ok(()),
                (_, KeyCode::Esc) => {
                    app.set_mode(Mode::Normal);
                    prev = None;
                }
                (Mode::Normal | Mode::Visual, KeyCode::Char('d'))
                    if key.modifiers == KeyModifiers::CONTROL =>
                {
                    app.r#move(app.config.page)
                }
                (Mode::Normal | Mode::Visual, KeyCode::Char('u'))
                    if key.modifiers == KeyModifiers::CONTROL =>
                {
                    app.r#move(-app.config.page)
                }
                (Mode::Normal | Mode::Visual, KeyCode::Char('h')) => app.r#move(-1),
                (Mode::Normal | Mode::Visual, KeyCode::Char('j')) => app.r#move(16),
                (Mode::Normal | Mode::Visual, KeyCode::Char('k')) => app.r#move(-16),
                (Mode::Normal | Mode::Visual, KeyCode::Char('l')) => app.r#move(1),
                (Mode::Normal, KeyCode::Char('e')) => app.change_endian(),
                (Mode::Normal | Mode::Visual, KeyCode::Char('d')) => app.delete(),
                (Mode::Normal, KeyCode::Char('v')) => app.set_mode(Mode::Visual),
                (Mode::Normal | Mode::Visual, KeyCode::Char('r')) => app.set_mode(Mode::Replace),
                (Mode::Normal, KeyCode::Char('i')) => app.set_mode(Mode::Insert),
                (Mode::Normal, KeyCode::Char('a')) => {
                    app.r#move(1);
                    app.set_mode(Mode::Insert);
                }
                (mode @ (Mode::Replace | Mode::Insert), KeyCode::Char(c)) => {
                    match (prev, c.to_digit(16)) {
                        (None, Some(hex)) => {
                            app.set(hex as u8);
                            prev = Some(hex);
                        }
                        (Some(a), Some(b)) => {
                            app.set((a * 16 + b) as u8);
                            prev = None;

                            app.r#move(1);
                            app.set_mode(Mode::Normal);

                            if mode == Mode::Insert {
                                app.set_mode(Mode::Insert);
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
}

#[derive(Parser)]
struct Args {
    file: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let lua = Lua::new();

    let mut tm = TerminalManager::new()?;

    let height = tm.terminal.size()?.height;

    let app = App::new(args, height, &lua)?;

    run_draw_loop(&mut tm.terminal, app)?;
    Ok(())
}

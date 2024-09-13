mod app;
mod config;

use num_traits::ops::bytes::FromBytes;
use std::{
    io::{stdout, Stdout},
    ops::Range,
    path::PathBuf,
    str::FromStr,
};

use app::{App, Highlight, Mode, Popup, Selection};
use clap::Parser;
use config::Endian;
use mlua::Lua;
use ratatui::{
    backend::{Backend, CrosstermBackend},
    crossterm::{
        event::{self, Event, KeyCode, KeyModifiers},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Padding, Paragraph},
    Frame, Terminal,
};

use anyhow::Result;

const INDEX: u16 = 9;
const HEX: u16 = (2 + 1) * 16 + 1;
const TEXT: u16 = 16;
const MAIN: u16 = INDEX + HEX + TEXT + 8;
const RIGHT: u16 = 38;

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

fn ui_index(f: &mut Frame, app: &App, area: Rect) {
    let current = app.single_selection() / 16;

    let range = app.visible_range();
    let start = range.start / 16;
    let end = (start + app.height as usize).min(range.end / 16 + 1);

    let mut lines = Vec::with_capacity(end - start + 1);
    lines.push(Line::raw(""));

    for index in start..end {
        let hex = format!("0x{index:05X}0");

        let style = if current == index {
            Style::default().bg(Color::White).fg(Color::Black)
        } else {
            Style::default()
        };

        lines.push(Line::styled(hex, style));
    }

    let widget = Text::from(lines);
    f.render_widget(widget, area);
}

fn ui_hex(f: &mut Frame, app: &App, area: Rect) {
    let range = app.visible_range();
    let offset = range.start;
    let data = &app.data[range];
    let selection = &app.selection;

    let mut hexes = Vec::with_capacity(app.height as usize);

    let header = Line::from(" 0  1  2  3  4  5  6  7   8  9  a  b  c  d  e  f");
    hexes.push(header);

    for (y, chunk) in data.chunks(16).enumerate() {
        let mut line = Vec::with_capacity(32);

        for (x, byte) in chunk.iter().enumerate() {
            let pos = offset + y * 16 + x;

            if x == 8 {
                line.push(Span::raw("  "));
            } else if x != 0 {
                let style = get_style(&app.highlights, (pos - 1)..pos, selection);
                line.push(Span::styled(" ", style));
            }

            let hex = format!("{byte:02x}");
            let style = get_style(&app.highlights, pos..pos, selection);
            line.push(Span::styled(hex, style));
        }

        hexes.push(Line::from(line));
    }

    let widget = Text::from(hexes);
    f.render_widget(widget, area);
}

fn ui_text(f: &mut Frame, app: &App, area: Rect) {
    let range = app.visible_range();
    let offset = range.start;
    let data = &app.data[range];

    let mut lines = Vec::with_capacity(app.height as usize);
    lines.push(Line::raw(""));

    for (y, chunk) in data.chunks(16).enumerate() {
        let mut line = Vec::with_capacity(8);

        for (x, byte) in chunk.iter().enumerate() {
            let pos = offset + y * 16 + x;

            let string = match byte {
                &c if c > 32 && c < 127 => format!("{}", c as char),
                _ => String::from("."),
            };

            let style = get_style(&app.highlights, pos..pos, &app.selection);
            line.push(Span::styled(string, style));
        }

        lines.push(Line::from(line));
    }

    let widget = Text::from(lines);
    f.render_widget(widget, area);
}

fn ui_info_endian(f: &mut Frame, app: &App, area: Rect) {
    let (selected, not_selected) = (
        Style::default().bg(Color::White).fg(Color::Black),
        Style::default().fg(Color::White),
    );

    let (big, little) = match app.config.endian {
        Endian::Big => (selected, not_selected),
        Endian::Little => (not_selected, selected),
    };

    // TODO: mabye center this?
    let widget = Line::from(vec![
        Span::styled(" Big ", big),
        Span::styled(" Little ", little),
    ]);

    f.render_widget(widget, area);
}

fn ui_info(f: &mut Frame, app: &mut App, area: Rect) {
    let data = &app.data;
    let config = &app.config;
    let current = app.single_selection();

    let byte = data[current];

    let [endian, info] = Layout::vertical([Constraint::Length(2), Constraint::Min(0)]).areas(area);

    ui_info_endian(f, app, endian);

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

    let mut string = match &app.selection {
        Selection::Single(_) => String::from(byte as char),
        Selection::Visual { range, .. } => match std::str::from_utf8(&data[range.clone()]) {
            Ok(s) => s.to_string(),
            Err(_) => String::from("* not utf8 *"),
        },
    };
    string.truncate(20);

    let table = [
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

    let lines = table
        .into_iter()
        .map(|(name, value)| Line::from(format!("{name:<8}: {value}")))
        .collect::<Vec<_>>();

    let widget = Text::from(lines);
    f.render_widget(widget, info);
}

fn ui_highlights(f: &mut Frame, app: &mut App, area: Rect) {
    let current = app.single_selection();

    let lines = app
        .highlights
        .iter()
        .filter(|hl| current >= hl.start && current < hl.end)
        .map(|hl| {
            let mut style = Style::default();

            if let Some(bg) = hl.bg {
                style = style.bg(bg);
            }
            if let Some(fg) = hl.fg {
                style = style.fg(fg);
            }

            Line::styled(&hl.text, style)
        })
        .collect::<Vec<_>>();

    let widget = Text::from(lines);
    f.render_widget(widget, area);
}

fn ui_header(f: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .padding(Padding::horizontal(2))
        .title(" Lazyhex ");

    let mode = Span::from(format!("{:?}", app.mode));
    let mut spans = vec![mode];

    if let Some(path) = &app.path {
        if let Some(last) = path.components().last() {
            spans.push(Span::raw(" | "));
            spans.push(Span::from(format!("{:?}", last.as_os_str())));
        }
    }

    let inner = block.inner(area);
    let text = Line::from(spans);

    f.render_widget(text, inner);
    f.render_widget(block, area);
}

fn ui_right(f: &mut Frame, app: &mut App, area: Rect) {
    let height = area.height;

    let right = if height > 20 + 20 {
        vec![Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]
    } else {
        vec![Constraint::Ratio(1, 1)]
    };

    let rights = Layout::vertical(right).split(area);

    let block = Block::default()
        .borders(Borders::ALL)
        .padding(Padding::uniform(1))
        .title(" Info ");
    let area = block.inner(rights[0]);

    ui_info(f, app, area);
    f.render_widget(block, rights[0]);

    if height > 40 {
        let block = Block::default()
            .borders(Borders::ALL)
            .padding(Padding::uniform(1))
            .title(" Highlights ");
        let area = block.inner(rights[1]);

        ui_highlights(f, app, area);
        f.render_widget(block, rights[1]);
    }
}

fn ui_primary(f: &mut Frame, app: &mut App, area: Rect) {
    let width = f.area().width;

    let constraints = if width > MAIN + RIGHT {
        vec![Constraint::Length(MAIN), Constraint::Length(RIGHT)]
    } else {
        vec![Constraint::Length(MAIN)]
    };

    let main = Layout::horizontal(constraints).split(area);

    if width > MAIN + RIGHT {
        ui_right(f, app, main[1]);
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .padding(Padding::horizontal(1))
        .title(" Hex ");

    let inner = block.inner(main[0]);

    let [index, hex, text] = Layout::horizontal([
        Constraint::Length(INDEX),
        Constraint::Length(HEX),
        Constraint::Length(TEXT),
    ])
    .flex(Flex::SpaceBetween)
    .areas(inner);

    app.height = inner.height - 1;

    ui_index(f, app, index);
    ui_hex(f, app, hex);
    ui_text(f, app, text);

    f.render_widget(block, main[0]);
}

fn ui_popup(f: &mut Frame, app: &mut App) {
    let [area] = Layout::horizontal([Constraint::Length(MAIN / 3)])
        .flex(Flex::Center)
        .areas(f.area());
    let [area] = Layout::vertical([Constraint::Length(3)])
        .flex(Flex::Center)
        .areas(area);

    let (title, data) = match &app.popup {
        Popup::None => ("", ""),
        Popup::Filename(filename) => ("Filename", filename.as_str()),
    };

    let popup = Paragraph::new(data).block(
        Block::bordered()
            .padding(Padding::horizontal(2))
            .title(title),
    );

    f.render_widget(Clear, area);
    f.render_widget(popup, area);
}

fn ui(f: &mut Frame, app: &mut App) {
    let width = f.area().width;

    let size = if width > MAIN + RIGHT {
        MAIN + RIGHT
    } else {
        MAIN
    };

    let [center] = Layout::horizontal([Constraint::Length(size)])
        .flex(Flex::Center)
        .areas(f.area());

    let [header, primary] =
        Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).areas(center);

    ui_header(f, app, header);
    ui_primary(f, app, primary);

    match app.popup {
        Popup::None => {}
        Popup::Filename(_) => ui_popup(f, app),
    }
}

fn event_main(app: &mut App) -> Result<bool> {
    #[allow(clippy::single_match)]
    match event::read()? {
        Event::Key(key) => match (app.mode, key.code) {
            (_, KeyCode::Char('q')) => return Ok(true),
            (_, KeyCode::Esc) => {
                app.set_mode(Mode::Normal);
                app.input = None;
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
            (Mode::Normal | Mode::Visual, KeyCode::Char('g')) => app.position(0),
            (Mode::Normal | Mode::Visual, KeyCode::Char('G')) => app.position(app.data.len() - 1),
            (Mode::Normal, KeyCode::Char('w')) => match &app.path {
                None => {
                    app.popup = Popup::Filename("".into());
                }
                Some(path) => app.write(path.clone()),
            },
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
                match (app.input, c.to_digit(16)) {
                    (None, Some(hex)) => {
                        app.set(hex as u8);
                        app.input = Some(hex);
                    }
                    (Some(a), Some(b)) => {
                        app.set((a * 16 + b) as u8);
                        app.input = None;

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

    Ok(false)
}

fn event_filename(app: &mut App) -> Result<bool> {
    let Popup::Filename(ref mut name) = &mut app.popup else {
        return Ok(false);
    };

    if let Event::Key(key) = event::read()? {
        match key.code {
            KeyCode::Enter => {
                let path = PathBuf::from(name.clone());
                app.write(path);
                app.popup = Popup::None;
            }
            KeyCode::Esc => {
                app.popup = Popup::None;
            }
            KeyCode::Char(ch) => name.push(ch),
            KeyCode::Backspace => {
                name.pop();
            }
            _ => {}
        }
    }

    Ok(false)
}

fn run_draw_loop<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        let quit = match app.popup {
            Popup::None => event_main(&mut app)?,
            Popup::Filename(_) => event_filename(&mut app)?,
        };

        if quit {
            return Ok(());
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

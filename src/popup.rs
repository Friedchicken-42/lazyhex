use std::{ops::Range, str::FromStr};

use crossterm::event::{Event, KeyCode};
use ratatui::{
    layout::{Constraint, Flex, Layout},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Padding, Paragraph, Wrap},
    Frame,
};

use crate::{
    app::{Highlight, Mode},
    command::{ClosePopup, Command, CreateHighlight, OpenPopup, Save, SetMode},
};

pub trait Popup {
    fn handle(&mut self, event: Event) -> Vec<Box<dyn Command>>;
    fn ui(&self, f: &mut Frame);
}

pub struct ViewOnly {
    pub title: String,
    pub content: String,
}

impl Popup for ViewOnly {
    fn handle(&mut self, event: Event) -> Vec<Box<dyn Command>> {
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => vec![Box::new(ClosePopup)],
                _ => vec![],
            }
        } else {
            vec![]
        }
    }

    fn ui(&self, f: &mut Frame) {
        let frame = f.area();

        let width = frame.width / 2;
        let height = frame.height / 2;

        let [area] = Layout::horizontal([Constraint::Length(width)])
            .flex(Flex::Center)
            .areas(f.area());
        let [area] = Layout::vertical([Constraint::Length(height)])
            .flex(Flex::Center)
            .areas(area);

        let popup = Paragraph::new(self.content.clone())
            .block(
                Block::bordered()
                    .padding(Padding::horizontal(2))
                    .title(format!(" {} ", self.title)),
            )
            .wrap(Wrap { trim: true });

        f.render_widget(Clear, area);
        f.render_widget(popup, area);
    }
}

pub struct Filename(String);

impl Filename {
    pub fn new() -> Self {
        Self(String::new())
    }
}

impl Popup for Filename {
    fn handle(&mut self, event: Event) -> Vec<Box<dyn Command>> {
        let close = Box::new(ClosePopup);

        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Enter => return vec![close, Box::new(Save(self.0.clone()))],
                KeyCode::Esc => return vec![close],
                KeyCode::Backspace => {
                    self.0.pop();
                }
                KeyCode::Char(ch) => self.0.push(ch),
                _ => {}
            }
        }

        vec![]
    }

    fn ui(&self, f: &mut Frame) {
        let frame = f.area();

        let width = frame.width / 2;
        let height = 3;

        let [area] = Layout::horizontal([Constraint::Length(width)])
            .flex(Flex::Center)
            .areas(f.area());
        let [area] = Layout::vertical([Constraint::Length(height)])
            .flex(Flex::Center)
            .areas(area);

        let popup = Paragraph::new(self.0.clone())
            .block(
                Block::bordered()
                    .padding(Padding::horizontal(2))
                    .title(" Filename "),
            )
            .wrap(Wrap { trim: true });

        f.render_widget(Clear, area);
        f.render_widget(popup, area);
    }
}

pub struct Overwrite(pub String);

impl Popup for Overwrite {
    fn handle(&mut self, event: Event) -> Vec<Box<dyn Command>> {
        let close = Box::new(ClosePopup);

        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Enter | KeyCode::Char('y') => {
                    return vec![close, Box::new(Save(self.0.clone()))]
                }
                KeyCode::Esc | KeyCode::Char('n' | 'q') => return vec![close],
                _ => {}
            }
        }

        vec![]
    }

    fn ui(&self, f: &mut Frame) {
        let frame = f.area();

        let width = frame.width / 2;
        let height = 3;

        let [area] = Layout::horizontal([Constraint::Length(width)])
            .flex(Flex::Center)
            .areas(f.area());
        let [area] = Layout::vertical([Constraint::Length(height)])
            .flex(Flex::Center)
            .areas(area);

        let popup = Paragraph::new(self.0.clone())
            .block(
                Block::bordered()
                    .padding(Padding::horizontal(2))
                    .title(" Overwrite? [Y/n] "),
            )
            .wrap(Wrap { trim: true });

        f.render_widget(Clear, area);
        f.render_widget(popup, area);
    }
}

#[derive(PartialEq, Eq)]
enum HSelector {
    Background,
    Foreground,
    Text,
    Ok,
    Cancel,
}

pub struct NewHighlight {
    start: usize,
    end: usize,
    bg: String,
    fg: String,
    text: String,
    current: HSelector,
}

impl NewHighlight {
    pub fn new(range: Range<usize>) -> Self {
        Self {
            start: range.start,
            end: range.end,
            bg: "#ffffff".into(),
            fg: "#000000".into(),
            text: String::new(),
            current: HSelector::Background,
        }
    }
}

impl Popup for NewHighlight {
    fn handle(&mut self, event: Event) -> Vec<Box<dyn Command>> {
        if let Event::Key(key) = event {
            match (&self.current, key.code) {
                (_, KeyCode::Esc) => return vec![Box::new(ClosePopup)],
                (mode, KeyCode::Char('q')) if *mode != HSelector::Text => {
                    return vec![Box::new(ClosePopup)]
                }
                (HSelector::Cancel, KeyCode::Enter) => return vec![Box::new(ClosePopup)],

                (HSelector::Background, KeyCode::Char(ch @ ('0'..='9' | 'a'..='f'))) => {
                    self.bg.push(ch);
                }
                (HSelector::Background, KeyCode::Backspace) if self.bg.len() > 1 => {
                    self.bg.pop();
                }
                (HSelector::Foreground, KeyCode::Char(ch @ ('0'..='9' | 'a'..='f'))) => {
                    self.fg.push(ch);
                }
                (HSelector::Foreground, KeyCode::Backspace) if self.fg.len() > 1 => {
                    self.fg.pop();
                }
                (HSelector::Text, KeyCode::Char(ch)) => {
                    self.text.push(ch);
                }
                (HSelector::Text, KeyCode::Backspace) => {
                    self.text.pop();
                }

                (HSelector::Ok, KeyCode::Enter) => {
                    let Ok(bg) = Color::from_str(&self.bg) else {
                        let popup = ViewOnly {
                            title: "Highligh Error".into(),
                            content: format!("Error parsing background color: {:?}", self.bg),
                        };
                        return vec![Box::new(OpenPopup::new(Box::new(popup)))];
                    };
                    let Ok(fg) = Color::from_str(&self.fg) else {
                        let popup = ViewOnly {
                            title: "Highligh Error".into(),
                            content: format!("Error parsing foregroung color: {:?}", self.fg),
                        };
                        return vec![Box::new(OpenPopup::new(Box::new(popup)))];
                    };

                    let hightlight = Highlight {
                        start: self.start,
                        end: self.end,
                        bg: Some(bg),
                        fg: Some(fg),
                        text: self.text.to_string(),
                    };

                    return vec![
                        Box::new(ClosePopup),
                        Box::new(CreateHighlight(hightlight)),
                        Box::new(SetMode::new(Mode::Normal)),
                    ];
                }

                (HSelector::Background, KeyCode::Down | KeyCode::Tab) => {
                    self.current = HSelector::Foreground
                }
                (HSelector::Foreground, KeyCode::Up) => self.current = HSelector::Background,
                (HSelector::Foreground, KeyCode::Down | KeyCode::Tab) => {
                    self.current = HSelector::Text
                }
                (HSelector::Text, KeyCode::Up) => self.current = HSelector::Foreground,
                (HSelector::Text, KeyCode::Down | KeyCode::Tab) => self.current = HSelector::Ok,
                (HSelector::Ok, KeyCode::Up) => self.current = HSelector::Text,
                (HSelector::Ok, KeyCode::Down | KeyCode::Right | KeyCode::Tab) => {
                    self.current = HSelector::Cancel
                }
                (HSelector::Cancel, KeyCode::Up | KeyCode::Left) => self.current = HSelector::Ok,

                _ => {}
            }
        }
        vec![]
    }

    fn ui(&self, f: &mut Frame) {
        let width = f.area().width / 2;
        let height = 12;

        let [area] = Layout::horizontal([Constraint::Length(width)])
            .flex(Flex::Center)
            .areas(f.area());
        let [area] = Layout::vertical([Constraint::Length(height)])
            .flex(Flex::Center)
            .areas(area);

        let default = Style::default();
        let selected = Style::default().fg(Color::Black).bg(Color::White);

        let create_line = |left: String, right: String, current: HSelector| -> Vec<Span> {
            vec![
                Span::raw(left).style(if self.current == current {
                    selected
                } else {
                    default
                }),
                Span::raw("  "),
                Span::from(right),
            ]
        };

        let color = default
            .bg(Color::from_str(&self.bg).unwrap_or(Color::Black))
            .fg(Color::from_str(&self.fg).unwrap_or(Color::White));

        let lines = [
            Line::from(format!(
                "Range:      0x{:04x} .. 0x{:04x}",
                self.start, self.end
            )),
            Line::from(create_line(
                "Background".into(),
                self.bg.to_string(),
                HSelector::Background,
            )),
            Line::from(create_line(
                "Foreground".into(),
                self.fg.to_string(),
                HSelector::Foreground,
            )),
            Line::from(create_line(
                "Text      ".into(),
                format!("\"{}\"", self.text),
                HSelector::Text,
            )),
            Line::raw(""),
            Line::from(vec![
                Span::raw("Example: "),
                Span::raw("de ad be ef").style(color),
            ]),
            Line::raw(""),
            Line::from(
                [
                    vec![Span::raw("   ")],
                    create_line("Ok".into(), "  ".into(), HSelector::Ok),
                    create_line("Cancel".into(), "  ".into(), HSelector::Cancel),
                ]
                .concat(),
            ),
        ];

        let block = Block::default()
            .borders(Borders::ALL)
            .padding(Padding::uniform(1))
            .title(" New Highlight ");

        let inner = block.inner(area);

        let widget = Text::from(lines.to_vec());

        f.render_widget(Clear, area);
        f.render_widget(block, area);
        f.render_widget(widget, inner);
    }
}

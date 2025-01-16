use crossterm::event::{Event, KeyCode};
use ratatui::{
    layout::{Constraint, Flex, Layout},
    widgets::{Block, Clear, Padding, Paragraph, Wrap},
    Frame,
};

use crate::command::{ClosePopup, Command, Save};

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

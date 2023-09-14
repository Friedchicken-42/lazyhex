use crate::app::{App, Mode};
mod hex;
use hex::hex;

mod index;
use index::index;

mod table;
use table::table;

mod info;
use info::info;

use ratatui::{layout::Constraint::*, prelude::*, widgets::*};

pub fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Length(3), Min(0)])
        .split(f.size());

    let mode = match app.mode {
        Mode::Normal => "NORMAL",
        Mode::Insert => "INSERT",
        Mode::Visual => "VISUAL",
    };

    let file = app.filename.unwrap_or("");

    let header = Paragraph::new(format!("  {mode}  |  {file}"))
        .block(Block::default().title("Lazyhex").borders(Borders::ALL));

    f.render_widget(header, layout[0]);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![Length(82), Min(0)])
        .split(layout[1]);

    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![Length(11), Length(52), Length(19)])
        .split(body[0]);

    let height = (body[0].height - 3) as usize;
    f.render_widget(index(app, height), main[0]);
    f.render_widget(hex(app, height), main[1]);
    f.render_widget(table(app, height), main[2]);

    let block = Block::default().borders(Borders::ALL).title("Hex");
    f.render_widget(block, body[0]);

    f.render_widget(info(app), body[1])
}

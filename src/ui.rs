use crate::app::App;
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
    let rects = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![Length(82), Min(0)])
        .split(f.size());

    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![Length(11), Length(52), Length(19)])
        .split(rects[0]);

    f.render_widget(index(app), main[0]);
    f.render_widget(hex(app), main[1]);
    f.render_widget(table(app), main[2]);

    let block = Block::default().borders(Borders::ALL).title("Hex");
    f.render_widget(block, rects[0]);

    f.render_widget(info(app), rects[1])
}

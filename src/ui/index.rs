use ratatui::{
    prelude::Alignment,
    text::Line,
    widgets::{Block, Padding, Paragraph, Widget},
};

use crate::app::App;

pub fn index(app: &App) -> impl Widget {
    let indexes: Vec<_> = (0..app.data.chunks(16).len())
        .map(|i| Line::from(format!("0x{i:05X}0")))
        .collect();

    Paragraph::new(indexes)
        .block(Block::default().padding(Padding {
            left: 1,
            right: 1,
            top: 2,
            bottom: 1,
        }))
        .alignment(Alignment::Right)
}

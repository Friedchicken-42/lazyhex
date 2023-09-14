use ratatui::{
    prelude::Alignment,
    text::Line,
    widgets::{Block, Padding, Paragraph, Widget},
};

use crate::app::App;

pub fn table(app: &App, height: usize) -> impl Widget {
    let skip = if app.selection.end / 16 > height - 1 {
        app.selection.end / 16 + 1 - height
    } else {
        0
    };

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
        .skip(skip)
        .take(height)
        .collect();

    Paragraph::new(table)
        .block(Block::default().padding(Padding {
            left: 1,
            right: 1,
            top: 2,
            bottom: 1,
        }))
        .alignment(Alignment::Left)
}

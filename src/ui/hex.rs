use ratatui::{
    prelude::Alignment,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph, Widget},
};

use crate::app::App;

fn convert(x: usize) -> (usize, usize) {
    let col = x / 16;
    let row = x % 16;
    let row = row * 2 + if row < 8 { 0 } else { 1 };

    (col, row)
}

pub fn hex(app: &App) -> impl Widget {
    let mut spans: Vec<_> = app
        .data
        .chunks(16)
        .map(|chunk| chunk.iter().map(|n| Span::from(format!("{n:02x}"))))
        .map(|chunk| {
            let len = chunk.len();
            let chunk = chunk.chain((len..16).map(|_| Span::raw("  ")));
            let mut chunk: Vec<_> = chunk.flat_map(|span| [span, Span::raw(" ")]).collect();
            chunk.insert(15, Span::raw(" "));

            chunk
        })
        .collect();

    let highlights = vec![(app.selected..=app.selected, Color::DarkGray)];

    for (highlight, color) in highlights {
        for (i, selected) in highlight.enumerate() {
            let (col, row) = convert(selected);
            spans[col][row].patch_style(Style::default().bg(color));

            if i != 0 {
                let (colp, rowp) = convert(selected - 1);

                if col == colp && row - rowp == 2 {
                    spans[col][row - 1].patch_style(Style::default().bg(color));
                }
            }
        }
    }

    let spans = spans.into_iter().map(|chunk| Line::from(chunk));

    let mut header: Vec<_> = (0..16).map(|i| Span::from(format!(" {i:x} "))).collect();
    header.insert(8, Span::raw(" "));
    let header = Line::from(header);
    let spans: Vec<_> = [header].into_iter().chain(spans).collect();

    Paragraph::new(spans)
        .block(
            Block::default()
                .padding(Padding::uniform(1))
                .borders(Borders::RIGHT | Borders::LEFT),
        )
        .alignment(Alignment::Center)
}

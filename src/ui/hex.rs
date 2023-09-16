use ratatui::{
    prelude::Alignment,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::viewer::{Highlight, Viewer};

fn convert(x: usize) -> (usize, usize) {
    let col = x / 16;
    let row = x % 16;
    let row = row * 2 + if row < 8 { 0 } else { 1 };

    (col, row)
}

pub fn hex<'a>(viewer: &Viewer, height: usize) -> Paragraph<'a> {
    let mut spans: Vec<_> = viewer
        .data
        .chunks(16)
        .map(|chunk| {
            chunk.iter().map(|n| match n {
                Some(x) => Span::from(format!("{x:02x}")),
                None => Span::raw("  "),
            })
        })
        .map(|chunk| {
            let len = chunk.len();
            let chunk = chunk.chain((len..16).map(|_| Span::raw("  ")));
            let mut chunk: Vec<_> = chunk.flat_map(|span| [span, Span::raw(" ")]).collect();
            chunk.insert(15, Span::raw(" "));

            chunk
        })
        .collect();

    let selection = [viewer.selection];
    let highlights = viewer.highlights.iter().chain(selection.iter());

    for Highlight { start, end, bg, fg } in highlights {
        for (i, selected) in (*start..=*end).enumerate() {
            let (col, row) = convert(selected);
            spans[col][row].patch_style(Style::default().bg(*bg).fg(*fg));

            if i != 0 {
                let (colp, rowp) = convert(selected - 1);

                if col == colp && row - rowp == 2 {
                    spans[col][row - 1].patch_style(Style::default().bg(*bg).fg(*fg));
                }
            }
        }
    }

    let spans = spans.into_iter().map(|chunk| Line::from(chunk));

    let mut header: Vec<_> = (0..16).map(|i| Span::from(format!(" {i:x} "))).collect();
    header.insert(8, Span::raw(" "));
    let header = Line::from(header);

    let skip = if viewer.selection.end / 16 > height - 1 {
        viewer.selection.end / 16 + 1 - height
    } else {
        0
    };

    let spans: Vec<_> = [header]
        .into_iter()
        .chain(spans.skip(skip).take(height))
        .collect();

    Paragraph::new(spans).alignment(Alignment::Center)
}

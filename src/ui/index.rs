use ratatui::{prelude::Alignment, style::Style, text::Line, widgets::Paragraph};

use crate::viewer::Viewer;

pub fn index<'a>(viewer: &Viewer, height: usize) -> Paragraph<'a> {
    let skip = if viewer.selection.end / 16 > height - 1 {
        viewer.selection.end / 16 + 1 - height
    } else {
        0
    };

    let indexes: Vec<_> = (0..viewer.data.chunks(16).len())
        .map(|i| {
            let id = format!("0x{i:05X}0");
            if i >= viewer.selection.start / 16 && i <= viewer.selection.end / 16 {
                Line::styled(id, Style::default().bg(ratatui::style::Color::DarkGray))
            } else {
                Line::from(id)
            }
        })
        .skip(skip)
        .take(height)
        .collect();

    Paragraph::new(indexes).alignment(Alignment::Right)
}

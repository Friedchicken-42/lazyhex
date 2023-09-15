use ratatui::{
    prelude::Alignment,
    text::Line,
    widgets::{Block, Padding, Paragraph, Widget},
};

use crate::viewer::Viewer;

pub fn index(viewer: &Viewer, height: usize) -> impl Widget {
    let skip = if viewer.selection.end / 16 > height - 1 {
        viewer.selection.end / 16 + 1 - height
    } else {
        0
    };

    let indexes: Vec<_> = (0..viewer.data.chunks(16).len())
        .map(|i| Line::from(format!("0x{i:05X}0")))
        .skip(skip)
        .take(height)
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

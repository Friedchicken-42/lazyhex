use ratatui::widgets::{Block, Borders, List, ListItem, Padding, Widget};

use crate::viewer::Viewer;

fn slice(data: &[u8], offset: usize, length: usize) -> Vec<u8> {
    let mut v = vec![0; length];

    for (i, value) in v.iter_mut().enumerate().take(length) {
        *value = match data.get(i + offset) {
            Some(n) => *n,
            None => 0,
        };
    }

    v
}

pub fn info(viewer: &Viewer) -> impl Widget {
    let byte = slice(viewer.data, viewer.selection.start, 1)[0];

    let long = slice(viewer.data, viewer.selection.start, 8)
        .into_iter()
        .fold(0, |acc, x| (acc << 8) | u64::from(x));

    let string = slice(
        viewer.data,
        viewer.selection.start,
        viewer.selection.end - viewer.selection.start + 1,
    );
    let string: String = string.iter().map(|c| *c as char).collect();

    let items = [
        ListItem::new(format!("hex:     0x{:02x}", byte)),
        ListItem::new(format!("binary:  0b{:b}", byte)),
        ListItem::new(format!("octal:   0o{:o}", byte)),
        ListItem::new(format!("i8:      {}", byte as i8)),
        ListItem::new(format!("u8:      {}", byte as u8)),
        ListItem::new(format!("i16:     {}", (long >> 48) as i16)),
        ListItem::new(format!("u16:     {}", (long >> 48) as u16)),
        ListItem::new(format!("i32:     {}", (long >> 32) as i32)),
        ListItem::new(format!("u32:     {}", (long >> 32) as u32)),
        ListItem::new(format!("i64:     {}", long as i64)),
        ListItem::new(format!("u64:     {}", long as u64)),
        ListItem::new(format!("f32:     {:.5e}", (long >> 32) as f32)),
        ListItem::new(format!("f64:     {:.5e}", long as f64)),
        ListItem::new(format!("char:    {}", byte as char)),
        ListItem::new(format!("string: {:?}", string)),
    ];

    List::new(items).block(
        Block::default()
            .title("Info")
            .borders(Borders::ALL)
            .padding(Padding::uniform(1)),
    )
}

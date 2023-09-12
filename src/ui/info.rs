use ratatui::widgets::{Block, Borders, List, ListItem, Padding, Widget};

use crate::app::App;

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

pub fn info(app: &App) -> impl Widget {
    let byte = slice(app.data, app.selection.start, 1)[0];

    let mut integer: u32 = 0;
    for b in slice(app.data, app.selection.start, 4) {
        integer = (integer << 8) | u32::from(b);
    }

    let items = [
        ListItem::new(format!("hex:   0x{:02x}", byte)),
        ListItem::new(format!("uint8: {}", byte)),
        ListItem::new(format!("char:  {}", byte as char)),
        ListItem::new(format!("int:   {}", integer)),
    ];
    List::new(items).block(
        Block::default()
            .title("Info")
            .borders(Borders::ALL)
            .padding(Padding::uniform(1)),
    )
}

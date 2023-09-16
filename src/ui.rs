use crate::{
    comparator::Comparator,
    viewer::{Mode, Viewer},
};
mod hex;
use hex::hex;

mod index;
use index::index;

mod table;
use table::table;

mod info;
use info::info;

use ratatui::{layout::Constraint::*, prelude::*, widgets::*};

pub fn viewer_ui<B: Backend>(f: &mut Frame<B>, viewer: &mut Viewer) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Length(3), Min(0)])
        .split(f.size());

    let mode = match viewer.mode {
        Mode::Normal => "NORMAL",
        Mode::Insert => "INSERT",
        Mode::Visual => "VISUAL",
    };

    let file = viewer.filename.unwrap_or("");

    let edited = if viewer.edited { "*" } else { "" };
    let header = Paragraph::new(format!("  {mode}  |  {file}{edited}"))
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

    let hextable = hex(viewer, height).block(
        Block::default()
            .padding(Padding::uniform(1))
            .borders(Borders::RIGHT | Borders::LEFT),
    );

    f.render_widget(index(viewer, height), main[0]);
    f.render_widget(hextable, main[1]);
    f.render_widget(table(viewer, height), main[2]);

    let block = Block::default().borders(Borders::ALL).title("Hex");
    f.render_widget(block, body[0]);

    f.render_widget(info(viewer), body[1])
}

pub fn comparator_ui<B: Backend>(f: &mut Frame<B>, comparator: &mut Comparator) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Length(3), Min(0)])
        .split(f.size());

    let file_old = comparator.viewer_old.filename.unwrap();
    let file_new = comparator.viewer_new.filename.unwrap();

    let mut header = vec![];

    if comparator.added > 0 {
        let added = Span::styled(
            format!("+{}", comparator.added),
            Style::default().bg(Color::Green),
        );
        header.push(added);
        header.push(Span::raw(" "));
    }

    if comparator.deleted > 0 {
        let deleted = Span::styled(
            format!("-{}", comparator.deleted),
            Style::default().bg(Color::Red),
        );
        header.push(deleted);
        header.push(Span::raw(" "));
    }

    if comparator.replaced > 0 {
        let replaced = Span::styled(
            format!("~{}", comparator.replaced),
            Style::default().bg(Color::Yellow).fg(Color::Black),
        );
        header.push(replaced);
        header.push(Span::raw(" "));
    }

    header.insert(
        0,
        Span::from(format!(" Comparing {file_old:?} and {file_new:?}")),
    );
    header.insert(1, Span::raw("  |  "));

    let header = Paragraph::new(Line::from(header))
        .block(Block::default().title("Lazyhex").borders(Borders::ALL));

    f.render_widget(header, layout[0]);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![Ratio(1, 2), Ratio(1, 2)])
        .split(layout[1]);

    let height = (body[0].height - 3) as usize;

    let old = hex(&comparator.viewer_old, height)
        .block(Block::default().title(file_old).borders(Borders::ALL));

    let new = hex(&comparator.viewer_new, height)
        .block(Block::default().title(file_new).borders(Borders::ALL));

    f.render_widget(old, body[0]);
    f.render_widget(new, body[1]);
}

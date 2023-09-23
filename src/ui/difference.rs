use ratatui::{
    style::Stylize,
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::comparator::Comparator;

fn diff<'a>(a: u8, b: u8) -> Vec<Span<'a>> {
    let d = a ^ b;

    (0..8)
        .rev()
        .map(|i| {
            if (d >> i) & 0b1 == 1 {
                if (b >> i) & 0b1 == 1 {
                    Span::from("1".green())
                } else {
                    Span::from("0".red())
                }
            } else {
                Span::raw(" ")
            }
        })
        .collect()
}

pub fn difference<'a>(comparator: &Comparator) -> Paragraph<'a> {
    let position = comparator.viewer_old.selection.start;
    let o = &comparator.viewer_old.data[position];
    let n = &comparator.viewer_new.data[position];

    let spans = match (*o, *n) {
        (None, None) => unreachable!(),
        (None, Some(new)) => {
            vec![
                vec![Span::raw("")],
                vec![Span::from(" ++++++ ".green())],
                vec![Span::from(format!("{new:08b}"))],
            ]
        }
        (Some(old), None) => {
            vec![
                vec![Span::from(format!("{old:08b}"))],
                vec![Span::from(" ------ ".red())],
                vec![Span::raw("")],
            ]
        }
        (Some(old), Some(new)) => {
            vec![
                vec![Span::from(format!("{old:08b}"))],
                diff(old, new),
                vec![Span::from(format!("{new:08b}"))],
            ]
        }
    };

    let lines: Vec<_> = spans.into_iter().map(|s| Line::from(s)).collect();

    Paragraph::new(lines)
}

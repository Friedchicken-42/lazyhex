use ratatui::style::Color;

#[derive(PartialEq)]
pub enum Mode {
    Normal,
    Insert,
    Visual,
}

#[derive(Clone, Copy)]
pub struct Highlight {
    pub start: usize,
    pub end: usize,
    pub bg: Color,
    pub fg: Color,
}

impl Highlight {
    pub fn set(&mut self, pos: usize) {
        self.start = pos;
        self.end = pos;
    }
}

pub struct Viewer<'a> {
    pub data: &'a mut Vec<Option<u8>>,
    pub selection: Highlight,
    pub filename: Option<&'a str>,
    pub mode: Mode,
    pub highlights: Vec<Highlight>,
    pub edited: bool,
}

const COLORS: [(Color, Color); 4] = [
    (Color::Red, Color::White),
    (Color::Green, Color::White),
    (Color::Yellow, Color::Black),
    (Color::Blue, Color::White),
];

impl<'a> Viewer<'a> {
    pub fn new(data: &'a mut Vec<Option<u8>>, filename: Option<&'a str>) -> Self {
        Self {
            selection: Highlight {
                start: 0,
                end: 0,
                bg: Color::DarkGray,
                fg: Color::White,
            },
            data,
            filename,
            mode: Mode::Normal,
            highlights: vec![],
            edited: false,
        }
    }

    pub fn left(&mut self) {
        if self.selection.end > 0 {
            self.selection.end -= 1;
        }

        if self.mode == Mode::Normal {
            self.selection.set(self.selection.end);
        } else if self.mode == Mode::Visual && self.selection.end < self.selection.start {
            self.selection.set(self.selection.start);
        }
    }

    pub fn right(&mut self) {
        if self.selection.end < self.data.len() - 1 {
            self.selection.end += 1;
        }

        if self.mode != Mode::Visual {
            self.selection.set(self.selection.end);
        }
    }

    pub fn up(&mut self) {
        if self.selection.end < 16 {
            self.selection.end = 0;
        } else {
            self.selection.end -= 16;
        }

        if self.mode != Mode::Visual {
            self.selection.set(self.selection.end);
        } else {
            self.selection.end = std::cmp::max(self.selection.start, self.selection.end);
        }
    }

    pub fn down(&mut self) {
        self.selection.end = std::cmp::min(self.data.len() - 1, self.selection.end + 16);

        if self.mode != Mode::Visual {
            self.selection.set(self.selection.end);
        }
    }

    pub fn set(&mut self, value: Option<u8>) {
        self.edited = true;
        for selected in self.selection.start..=self.selection.end {
            self.data[selected] = value;
        }
    }

    pub fn flush(&mut self) {
        if let Some(path) = &self.filename {
            let data: Vec<u8> = self
                .data
                .iter()
                .filter(|d| d.is_some())
                .map(|d| d.unwrap())
                .collect();

            let _ = std::fs::write(path, data);
            self.edited = false;
        }
    }

    pub fn append(&mut self) {
        self.edited = true;
        if self.selection.end + 1 < self.data.len() {
            self.data.insert(self.selection.end + 1, Some(0));
        } else {
            self.data.push(Some(0));
        }
    }

    pub fn delete(&mut self) {
        self.edited = true;
        self.data.drain(self.selection.start..=self.selection.end);

        let length = self.selection.end - self.selection.start + 1;

        self.highlights
            .retain(|h| h.start < self.selection.start || h.end > self.selection.end);

        for highlight in self.highlights.iter_mut() {
            if highlight.start > self.selection.start {
                highlight.start -= length;
            }

            if highlight.end >= self.selection.end {
                highlight.end -= length
            }
        }

        if self.data.is_empty() {
            self.data.push(Some(0));
        }

        self.selection.end = std::cmp::min(self.selection.end, self.data.len() - 1);
    }

    pub fn highlight(&mut self) {
        let prev = self
            .highlights
            .iter()
            .position(|h| h.start == self.selection.start && h.end == self.selection.end);

        if let Some(index) = prev {
            self.highlights.remove(index);
        } else {
            let (bg, fg) = COLORS[self.highlights.len() % COLORS.len()];

            self.highlights.push(Highlight {
                bg,
                fg,
                ..self.selection
            });
        }
    }
}

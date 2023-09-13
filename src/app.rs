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
    pub color: Color,
}

pub struct App<'a> {
    pub data: &'a mut Vec<u8>,
    pub selection: Highlight,
    pub filename: Option<&'a str>,
    pub mode: Mode,
    pub highlights: Vec<Highlight>,
}

const COLORS: [Color; 4] = [Color::Red, Color::Green, Color::Yellow, Color::Blue];

impl<'a> App<'a> {
    pub fn new(data: &'a mut Vec<u8>, filename: Option<&'a str>) -> Self {
        Self {
            selection: Highlight {
                start: 0,
                end: 0,
                color: Color::DarkGray,
            },
            data,
            filename,
            mode: Mode::Normal,
            highlights: vec![],
        }
    }

    pub fn left(&mut self) {
        if self.selection.end > 0 {
            self.selection.end -= 1;
        }

        if self.mode == Mode::Normal {
            self.selection.start = self.selection.end;
        } else if self.mode == Mode::Visual && self.selection.end < self.selection.start {
            self.selection.end = self.selection.start;
        }
    }

    pub fn right(&mut self) {
        if self.selection.end < self.data.len() - 1 {
            self.selection.end += 1;
        }

        if self.mode == Mode::Normal {
            self.selection.start = self.selection.end;
        }
    }

    pub fn up(&mut self) {
        if self.selection.end < 16 {
            self.selection.end = 0;
        } else {
            self.selection.end -= 16;
        }

        if self.mode == Mode::Normal {
            self.selection.start = self.selection.end;
        } else if self.mode == Mode::Visual {
            self.selection.end = std::cmp::max(self.selection.start, self.selection.end);
        }
    }

    pub fn down(&mut self) {
        self.selection.end = std::cmp::min(self.data.len() - 1, self.selection.end + 16);

        if self.mode == Mode::Normal {
            self.selection.start = self.selection.end;
        }
    }

    pub fn set(&mut self, value: u8) {
        for selected in self.selection.start..=self.selection.end {
            self.data[selected] = value;
        }
    }

    pub fn flush(&mut self) {
        if let Some(path) = &self.filename {
            let _ = std::fs::write(path, &self.data);
        }
    }

    pub fn append(&mut self) {
        if self.selection.end + 1 < self.data.len() {
            self.data.insert(self.selection.end + 1, 0);
        } else {
            self.data.push(0);
        }
    }

    pub fn delete(&mut self) {
        self.data.drain(self.selection.start..=self.selection.end);

        if self.data.is_empty() {
            self.data.push(0);
        }

        self.selection.end = std::cmp::min(self.selection.end, self.data.len() - 1);
    }

    pub fn highlight(&mut self) {
        let color = COLORS[self.highlights.len() % COLORS.len()];

        self.highlights.push(Highlight {
            color,
            ..self.selection
        });
    }
}

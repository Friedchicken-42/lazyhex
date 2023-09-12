#[derive(PartialEq)]
pub enum Mode {
    Normal,
    Insert,
    Visual,
}

pub struct Selection {
    pub start: usize,
    pub end: usize,
}

pub struct App<'a> {
    pub data: &'a mut Vec<u8>,
    pub selection: Selection,
    pub filename: Option<String>,
    pub mode: Mode,
}

impl<'a> App<'a> {
    pub fn new(data: &'a mut Vec<u8>, filename: Option<String>) -> Self {
        Self {
            selection: Selection { start: 0, end: 0 },
            data,
            filename,
            mode: Mode::Normal,
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

        if self.data.len() == 0 {
            self.data.push(0);
        }

        self.selection.end = std::cmp::min(self.selection.end, self.data.len() - 1);
    }
}

pub enum Mode {
    Normal,
    Insert,
}

pub struct App<'a> {
    pub data: &'a mut Vec<u8>,
    pub selected: usize,
    pub filename: Option<String>,
    pub mode: Mode,
}

impl<'a> App<'a> {
    pub fn new(data: &'a mut Vec<u8>, filename: Option<String>) -> Self {
        Self {
            selected: 0,
            data,
            filename,
            mode: Mode::Normal,
        }
    }

    pub fn left(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn right(&mut self) {
        if self.selected < self.data.len() - 1 {
            self.selected += 1;
        }
    }

    pub fn up(&mut self) {
        if self.selected < 16 {
            self.selected = 0;
        } else {
            self.selected -= 16;
        }
    }

    pub fn down(&mut self) {
        self.selected = std::cmp::min(self.data.len() - 1, self.selected + 16);
    }

    pub fn set(&mut self, value: u8) {
        self.data[self.selected] = value;
    }

    pub fn flush(&mut self) {
        if let Some(path) = &self.filename {
            let _ = std::fs::write(path, &self.data);
        }
    }

    pub fn append(&mut self) {
        if self.selected + 1 < self.data.len() {
            self.data.insert(self.selected + 1, 0);
        } else {
            self.data.push(0);
        }

        self.right();
    }

    pub fn delete(&mut self) {
        if self.data.len() > 1 {
            self.data.remove(self.selected);
        } else {
            self.data[0] = 0;
        }
    }
}

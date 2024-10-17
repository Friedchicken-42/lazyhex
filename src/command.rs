use std::{any::Any, fmt::Debug};

use crate::app::{App, HighlightUpdate, Mode, Selection};

pub trait Command: Any + Debug {
    fn execute(&mut self, app: &mut App);
    fn undo(&self, app: &mut App);
}

#[derive(Debug)]
pub struct SetMode {
    mode: Mode,
    selection: Selection,
}

impl SetMode {
    pub fn new(mode: Mode) -> Self {
        Self {
            mode,
            selection: Selection::Single(0),
        }
    }
}

impl Command for SetMode {
    fn execute(&mut self, app: &mut App) {
        std::mem::swap(&mut app.mode, &mut self.mode);
        self.selection = app.selection.clone();

        match app.mode {
            Mode::Normal => {
                let single = app.single_selection();
                app.selection = Selection::Single(single);
                app.input = None;
            }
            Mode::Visual => {
                app.selection = match &self.selection {
                    Selection::Single(current) => Selection::Visual {
                        current: *current,
                        center: *current,
                        range: *current..(*current + 1),
                    },
                    visual => visual.clone(),
                };
            }
            Mode::Replace => {}
            Mode::Insert => {
                app.execute(Insert);
            }
        }
    }

    fn undo(&self, app: &mut App) {
        app.mode = self.mode;
        app.selection = self.selection.clone();
    }
}

#[derive(Debug)]
pub struct Move(i32);

impl Move {
    pub fn new(offset: i32) -> Self {
        Self(offset)
    }
}

impl Command for Move {
    fn execute(&mut self, app: &mut App) {
        let increment = self.0;

        app.selection = match &app.selection {
            Selection::Single(current) => {
                let new = *current as i32 + increment;
                let new = 0.max(new).min(app.data.len() as i32 - 1);
                Selection::Single(new as usize)
            }
            Selection::Visual {
                current, center, ..
            } => {
                let new = *current as i32 + increment;
                let new = 0.max(new).min(app.data.len() as i32 - 1) as usize;
                let start = new.min(*center);
                let end = (new + 1).max(*center + 1);
                Selection::Visual {
                    current: new,
                    range: start..end,
                    center: *center,
                }
            }
        };
    }

    fn undo(&self, app: &mut App) {
        Move(-self.0).execute(app);
    }
}

#[derive(Debug)]
pub struct Position {
    new: usize,
    old: usize,
}

impl Position {
    pub fn new(pos: usize) -> Self {
        Position { new: pos, old: 0 }
    }
}

impl Command for Position {
    fn execute(&mut self, app: &mut App) {
        self.old = app.single_selection();

        let pos = self.new;
        let pos = pos.min(app.data.len() - 1);

        app.selection = match &app.selection {
            Selection::Single(_) => Selection::Single(pos),
            Selection::Visual { center, .. } => {
                let start = (*center).min(pos);
                let end = (*center).max(pos) + 1;
                Selection::Visual {
                    current: pos,
                    range: start..end,
                    center: *center,
                }
            }
        };
    }

    fn undo(&self, app: &mut App) {
        Self::new(self.old).execute(app);
    }
}

#[derive(Debug)]
pub struct Delete(Vec<u8>);

impl Delete {
    pub fn new() -> Self {
        Self(vec![])
    }
}

impl Command for Delete {
    fn execute(&mut self, app: &mut App) {
        let selection = app.selected();
        let deleted = app.data.drain(selection.clone()).collect();

        app.update_highlights(HighlightUpdate::Remove);

        let current = match &app.selection {
            Selection::Single(current) => *current,
            Selection::Visual { range, .. } => range.start,
        };

        if app.data.is_empty() {
            app.data.push(0);
        }

        let current = current.min(app.data.len() - 1);

        app.selection = Selection::Single(current);

        app.mode = Mode::Normal;

        app.edited = true;

        self.0 = deleted;
    }

    fn undo(&self, app: &mut App) {
        for value in &self.0 {
            Insert.execute(app);
            Set::new(*value).execute(app);
            Move(1).execute(app);
        }

        Move(-1).execute(app);
    }
}

#[derive(Debug)]
pub struct Set(Vec<u8>);

impl Set {
    pub fn new(value: u8) -> Self {
        Self(vec![value])
    }
}

impl Command for Set {
    fn execute(&mut self, app: &mut App) {
        let range = app.selected();
        let old = &app.data[range.clone()].to_vec();

        if self.0.len() == 1 {
            for i in range {
                app.data[i] = self.0[0];
            }
        } else if self.0.len() == range.len() {
            for (i, pos) in range.enumerate() {
                app.data[pos] = self.0[i];
            }
        } else {
            panic!("wrong range for `Set` and `selection`");
        }

        // TODO: should be skip
        app.update_highlights(HighlightUpdate::Add);
        self.0 = old.clone();
    }

    fn undo(&self, app: &mut App) {
        Self(self.0.clone()).execute(app);
    }
}

#[derive(Debug)]
pub struct Insert;

impl Command for Insert {
    fn execute(&mut self, app: &mut App) {
        let current = app.single_selection();
        app.data.insert(current, 0);
        app.update_highlights(HighlightUpdate::Add);
        app.edited = true;
    }

    fn undo(&self, app: &mut App) {
        Delete::new().execute(app);
    }
}

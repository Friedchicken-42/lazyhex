use std::any::Any;

use crate::app::{App, Mode, Selection};

pub trait Command: Any {
    fn execute(&mut self, app: &mut App);
    fn undo(&self, app: &mut App);
}

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

        app.move_highlights(selection.clone(), true);

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

pub struct Set(Vec<u8>);

impl Set {
    pub fn new(value: u8) -> Self {
        Self(vec![value])
    }
}

impl Command for Set {
    fn execute(&mut self, app: &mut App) {
        // let current = app.single_selection();
        // std::mem::swap(&mut app.data[current], &mut self.0);
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

        self.0 = old.clone();
    }

    fn undo(&self, app: &mut App) {
        Self(self.0.clone()).execute(app);
    }
}

pub struct Insert;

impl Command for Insert {
    fn execute(&mut self, app: &mut App) {
        let current = app.single_selection();
        app.data.insert(current, 0);
    }

    fn undo(&self, app: &mut App) {
        Delete::new().execute(app);
    }
}

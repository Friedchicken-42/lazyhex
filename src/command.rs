use std::{fmt::Debug, mem, path::PathBuf};

use crate::{
    app::{App, HighlightUpdate, Mode, Selection},
    popup::{Filename, Overwrite, Popup},
};

#[derive(PartialEq, Eq)]
pub enum HistoryStatus {
    Save,
    Skip,
    Merge,
}

pub trait Command {
    fn execute(&mut self, app: &mut App);
    fn undo(&self, app: &mut App);

    fn history_status(&self) -> HistoryStatus {
        HistoryStatus::Save
    }
}

pub struct Quit;

impl Command for Quit {
    fn execute(&mut self, app: &mut App) {
        app.quit = true;
    }

    fn undo(&self, _: &mut App) {
        unimplemented!()
    }

    fn history_status(&self) -> HistoryStatus {
        HistoryStatus::Skip
    }
}

pub struct Undo;

impl Command for Undo {
    fn execute(&mut self, app: &mut App) {
        while let Some(command) = app.history.pop() {
            command.undo(app);

            match (command.history_status(), app.history.last()) {
                (HistoryStatus::Merge, Some(next))
                    if next.history_status() == HistoryStatus::Merge => {}
                _ => break,
            }
        }
    }

    fn undo(&self, _: &mut App) {
        unimplemented!()
    }

    fn history_status(&self) -> HistoryStatus {
        HistoryStatus::Skip
    }
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
                app.execute(Box::new(Insert));
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

    fn history_status(&self) -> HistoryStatus {
        HistoryStatus::Merge
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
        app.edited = true;

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

pub struct OpenPopup(Option<Box<dyn Popup>>);

impl OpenPopup {
    pub fn new(popup: Box<dyn Popup>) -> Self {
        Self(Some(popup))
    }
}

impl Command for OpenPopup {
    fn execute(&mut self, app: &mut App) {
        let mut local = None;
        mem::swap(&mut self.0, &mut local);

        if let Some(popup) = local {
            app.set_popup(popup);
        }
    }

    fn undo(&self, _: &mut App) {}
}

pub struct ClosePopup;

impl Command for ClosePopup {
    fn execute(&mut self, app: &mut App) {
        app.clear_popup();
    }

    fn undo(&self, _: &mut App) {}
}

pub struct Write;

impl Command for Write {
    fn execute(&mut self, app: &mut App) {
        match &app.path {
            None => {
                let popup = Box::new(Filename::new());
                OpenPopup::new(popup).execute(app);
            }
            Some(path) => match path.exists() {
                true => {
                    let path = path.to_string_lossy().to_string();
                    let popup = Box::new(Overwrite(path));
                    OpenPopup::new(popup).execute(app);
                }
                false => app.write(),
            },
        }
    }

    fn undo(&self, _: &mut App) {}
}

pub struct Save(pub String);

impl Command for Save {
    fn execute(&mut self, app: &mut App) {
        app.path = Some(PathBuf::from(self.0.clone()));
        app.write();
    }

    fn undo(&self, _: &mut App) {}
}

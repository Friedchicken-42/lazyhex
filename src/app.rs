use std::{ops::Range, path::PathBuf, str::FromStr};

use anyhow::Result;
use mlua::{FromLua, Function, Lua, Table};
use ratatui::style::Color;

use crate::{
    config::{Config, Endian},
    Args,
};

#[derive(Debug, Clone)]
pub enum Selection {
    Single(usize),
    Visual {
        current: usize,
        range: Range<usize>,
        center: usize,
    },
}

#[derive(Debug)]
pub struct Highlight {
    pub start: usize,
    pub end: usize,
    pub bg: Option<Color>,
    pub fg: Option<Color>,
    pub text: String,
}

fn load_highlights(data: &[u8], lua: &Lua, callback: &Function) -> Result<Vec<Highlight>> {
    let mut highlights = vec![];

    let assert_range = |start: usize, end: usize| -> mlua::Result<()> {
        if start > end {
            return Err(mlua::Error::RuntimeError(format!(
                "`start` cannot be after than `end`: ({start} < {end})"
            )));
        }

        if end > data.len() {
            return Err(mlua::Error::RuntimeError(format!(
                "`end` ({end}) is outbound, max is {}",
                data.len(),
            )));
        }
        Ok(())
    };

    let out = lua.scope(|scope| {
        let buffer = lua.create_table()?;

        let color = scope.create_function_mut(|_, table: Table| {
            fn parse<T>(table: &Table, index: usize, name: &str) -> T
            where
                T: for<'a> FromLua<'a> + Default,
            {
                table
                    .get::<usize, T>(index)
                    .or(table.get::<&str, T>(name))
                    .unwrap_or_default()
            }

            let start = parse(&table, 1, "a");
            let end = match parse(&table, 2, "b") {
                0 => start + 1,
                x => x,
            };

            let bg: String = parse(&table, 3, "bg");

            let bg = match bg {
                x if x.is_empty() => None,
                color => {
                    let color = Color::from_str(&color).map_err(|_| {
                        mlua::Error::RuntimeError("cannot parse background into color".into())
                    })?;
                    Some(color)
                }
            };

            let fg: String = parse(&table, 4, "fg");
            let fg = match fg {
                x if x.is_empty() => None,
                color => {
                    let color = Color::from_str(&color).map_err(|_| {
                        mlua::Error::RuntimeError("cannot parse foreground into color".into())
                    })?;
                    Some(color)
                }
            };

            let text = parse(&table, 5, "text");

            assert_range(start, end)?;

            highlights.push(Highlight {
                start,
                end,
                fg,
                bg,
                text,
            });

            Ok(())
        })?;

        let read = scope.create_function(|_, (start, end): (usize, usize)| {
            assert_range(start, end)?;
            Ok(&data[start..end])
        })?;

        let read_be = scope.create_function(|_, (start, end): (usize, Option<usize>)| {
            let end = end.unwrap_or(start + 1);
            assert_range(start, end)?;
            let mut out = 0;
            for byte in &data[start..end] {
                out <<= 8;
                out += *byte as i64;
            }
            Ok(out)
        })?;

        let read_le = scope.create_function(|_, (start, end): (usize, Option<usize>)| {
            let end = end.unwrap_or(start + 1);
            assert_range(start, end)?;
            let mut out = 0;
            for (i, byte) in data[start..end].iter().enumerate() {
                out += (*byte as i64) << i;
            }
            Ok(out)
        })?;

        buffer.set("color", color)?;
        buffer.set("read", read)?;
        buffer.set("read_be", read_be)?;
        buffer.set("read_le", read_le)?;

        callback.call::<Table, ()>(buffer)?;
        Ok(())
    });

    match out {
        Ok(_) => Ok(highlights),
        Err(_) => Ok(vec![]),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    Normal,
    Visual,
    Replace,
    Insert,
}

#[derive(PartialEq)]
pub enum Popup {
    None,
    Filename(String),
    FileErr(String),
    Overwrite(PathBuf),
}

pub trait Command {
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
        let mut command = Move(-self.0);
        command.execute(app);
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
        let mut command = Self::new(self.old);
        command.execute(app);
    }
}

pub struct Delete(Range<usize>, Vec<u8>);

impl Delete {
    pub fn new() -> Self {
        Self(0..0, vec![])
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
        app.set_mode(Mode::Normal);
        app.edited = true;

        self.0 = selection;
        self.1 = deleted;
    }

    fn undo(&self, app: &mut App) {
        todo!()
    }
}

pub struct App<'lua> {
    pub data: Vec<u8>,
    pub path: Option<PathBuf>,
    pub config: Config<'lua>,
    pub selection: Selection,
    pub highlights: Vec<Highlight>,
    pub history: Vec<Box<dyn Command>>,

    pub height: u16,
    pub mode: Mode,
    pub popup: Popup,
    pub input: Option<u32>,
    pub edited: bool,
}

impl<'lua> App<'lua> {
    pub fn new(args: Args, height: u16, lua: &'lua Lua) -> Result<Self> {
        let config = Config::load(lua)?;

        let (data, path) = match args.file {
            Some(path) => (std::fs::read(&path)?, Some(PathBuf::from(path))),
            None => (vec![0], None),
        };

        let highlights = load_highlights(&data, lua, &config.highlight)?;

        Ok(Self {
            data,
            path,
            config,
            selection: Selection::Single(0),
            highlights,
            history: vec![],

            height: height - 4,
            mode: Mode::Normal,
            popup: Popup::None,
            input: None,
            edited: false,
        })
    }

    pub fn execute(&mut self, mut command: impl Command + 'static) {
        command.execute(self);
        self.history.push(Box::new(command));
    }

    pub fn undo(&mut self) {
        if let Some(command) = self.history.pop() {
            command.undo(self)
        }
    }

    pub fn change_endian(&mut self) {
        self.config.endian = match self.config.endian {
            Endian::Little => Endian::Big,
            Endian::Big => Endian::Little,
        };
    }

    pub fn visible_range(&self) -> Range<usize> {
        let current = self.single_selection() / 16;

        let height = self.height as usize;
        let rows = self.data.len() / 16;

        let max_rows = rows.saturating_sub(height).saturating_add(1);

        let start = current.saturating_sub(height / 2).min(max_rows);
        let end = ((start + height) * 16).min(self.data.len());

        (start * 16)..end
    }

    pub fn single_selection(&self) -> usize {
        match self.selection {
            Selection::Single(selection) => selection,
            Selection::Visual { current, .. } => current,
        }
    }

    pub fn selected(&self) -> Range<usize> {
        match &self.selection {
            Selection::Single(sel) => *sel..(*sel + 1),
            Selection::Visual { range, .. } => range.clone(),
        }
    }

    pub fn set_mode(&mut self, mode: Mode) {
        if self.mode == mode {
            return;
        }

        self.mode = mode;

        match self.mode {
            Mode::Normal => {
                let single = self.single_selection();
                self.selection = Selection::Single(single);
            }
            Mode::Visual => {
                self.selection = match &self.selection {
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
                let current = self.single_selection();
                self.data.insert(current, 0);
                self.move_highlights(current..(current + 1), false);
            }
        }
    }

    pub fn set(&mut self, value: u8) {
        let selection = self.selected();

        for x in self.data[selection].iter_mut() {
            *x = value;
        }

        self.edited = true;
    }

    fn move_highlights(&mut self, range: Range<usize>, remove: bool) {
        for highlight in self.highlights.iter_mut() {
            if remove {
                if highlight.start >= range.start {
                    highlight.start = highlight.start.saturating_sub(range.len());
                }
                if highlight.end > range.start {
                    highlight.end = highlight.end.saturating_sub(range.len());
                }
            } else {
                if highlight.start >= range.start {
                    highlight.start += range.len();
                }
                if highlight.end > range.start {
                    highlight.end += range.len();
                }
            }
        }
    }

    pub fn delete(&mut self) {
        let range = self.selected();
        self.data.drain(range.clone());
    }

    pub fn write(&mut self, path: PathBuf) {
        match std::fs::write(&path, &self.data) {
            Ok(_) => {
                self.path = Some(path);
                self.popup = Popup::None;
                self.edited = false;
            }
            Err(e) => {
                self.popup = Popup::FileErr(format!("{e:?}"));
            }
        }
    }

    pub fn write_ask(&mut self, path: PathBuf) {
        match path.exists() {
            true => {
                self.popup = Popup::Overwrite(path);
            }
            false => self.write(path),
        }
    }
}

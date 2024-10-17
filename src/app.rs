use std::{any::TypeId, ops::Range, path::PathBuf, str::FromStr};

use anyhow::Result;
use mlua::{FromLua, Function, Lua, Table};
use ratatui::style::Color;

use crate::{
    command::{Command, Move},
    config::{Config, Endian, HighlightOnDelete},
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

// TODO: find a way to handle error with `Popup::Error`
fn load_highlights(data: &[u8], lua: &Lua, callback: &Function) -> Vec<Highlight> {
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
        Ok(_) => highlights,
        Err(_) => vec![],
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
    Filename(String),
    Error { title: String, content: String },
    Overwrite(PathBuf),
}

pub enum HighlightUpdate {
    Add,
    Remove,
}

pub struct App<'lua> {
    pub data: Vec<u8>,
    pub path: Option<PathBuf>,
    pub config: Config<'lua>,
    pub selection: Selection,
    pub highlights: Vec<Highlight>,
    pub history: Vec<Box<dyn Command>>,
    pub lua: &'lua Lua,

    pub height: u16,
    pub mode: Mode,
    pub popup: Option<Popup>,
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

        let highlights = load_highlights(&data, lua, &config.highlight);

        Ok(Self {
            data,
            path,
            config,
            selection: Selection::Single(0),
            highlights,
            history: vec![],
            lua,

            height: height - 4,
            mode: Mode::Normal,
            popup: None,
            input: None,
            edited: false,
        })
    }

    pub fn set_popup(&mut self, popup: Popup) {
        self.popup = Some(popup);
    }

    pub fn clear_popup(&mut self) {
        self.popup = None;
    }

    pub fn execute(&mut self, mut command: impl Command + 'static) {
        command.execute(self);
        self.history.push(Box::new(command));
    }

    pub fn undo(&mut self) {
        loop {
            let Some(command) = self.history.pop() else {
                return;
            };

            command.undo(self);

            if command.type_id() != TypeId::of::<Move>() {
                break;
            }
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

    pub fn update_highlights(&mut self, update: HighlightUpdate) {
        match self.config.on_delete {
            HighlightOnDelete::Update => {
                let range = self.selected();
                self.move_highlights(range, update);
            }
            HighlightOnDelete::Reload => {
                self.highlights = load_highlights(&self.data, self.lua, &self.config.highlight);
            }
        }
    }

    fn move_highlights(&mut self, range: Range<usize>, update: HighlightUpdate) {
        for highlight in self.highlights.iter_mut() {
            match update {
                HighlightUpdate::Remove => {
                    if highlight.start >= range.start {
                        highlight.start = highlight.start.saturating_sub(range.len());
                    }
                    if highlight.end > range.start {
                        highlight.end = highlight.end.saturating_sub(range.len());
                    }
                }
                HighlightUpdate::Add => {
                    if highlight.start >= range.start {
                        highlight.start += range.len();
                    }
                    if highlight.end > range.start {
                        highlight.end += range.len();
                    }
                }
            }
        }
    }

    pub fn write(&mut self, path: PathBuf) {
        match std::fs::write(&path, &self.data) {
            Ok(_) => {
                self.path = Some(path);
                self.edited = false;
                self.clear_popup();
            }
            Err(e) => {
                let popup = Popup::Error {
                    title: "File Error".into(),
                    content: format!("{e:?}"),
                };
                self.set_popup(popup);
            }
        }
    }

    pub fn write_ask(&mut self, path: PathBuf) {
        match path.exists() {
            true => {
                let popup = Popup::Overwrite(path);
                self.set_popup(popup);
            }
            false => self.write(path),
        }
    }
}

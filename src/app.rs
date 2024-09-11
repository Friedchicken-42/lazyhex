use std::{ops::Range, str::FromStr};

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

pub struct App<'lua> {
    pub data: Vec<u8>,
    pub config: Config<'lua>,
    pub height: u16,
    pub selection: Selection,
    pub highlights: Vec<Highlight>,
    pub mode: Mode,
}

impl<'lua> App<'lua> {
    pub fn new(args: Args, height: u16, lua: &'lua Lua) -> Result<Self> {
        let config = Config::load(lua)?;

        let data = match args.file {
            Some(path) => std::fs::read(&path)?,
            None => vec![0],
        };

        let highlights = load_highlights(&data, lua, &config.highlight)?;

        Ok(Self {
            data,
            config,
            height: height - 4,
            selection: Selection::Single(0),
            highlights,
            mode: Mode::Normal,
        })
    }

    pub fn r#move(&mut self, increment: i32) {
        self.selection = match &self.selection {
            Selection::Single(current) => {
                let new = *current as i32 + increment;
                let new = 0.max(new).min(self.data.len() as i32 - 1);
                Selection::Single(new as usize)
            }
            Selection::Visual {
                current, center, ..
            } => {
                let new = *current as i32 + increment;
                let new = 0.max(new).min(self.data.len() as i32 - 1) as usize;
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

    pub fn set_mode(&mut self, mode: Mode) {
        if self.mode == mode {
            return;
        }

        self.mode = mode;

        match self.mode {
            Mode::Normal => {
                self.selection = match self.selection {
                    Selection::Single(s) => Selection::Single(s),
                    Selection::Visual { current, .. } => Selection::Single(current),
                };
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
        match &self.selection {
            Selection::Single(current) => self.data[*current] = value,
            Selection::Visual { range, .. } => {
                for x in self.data[range.clone()].iter_mut() {
                    *x = value;
                }
            }
        }
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
        let range = match &self.selection {
            Selection::Single(idx) => (*idx)..(*idx + 1),
            Selection::Visual { range, .. } => range.clone(),
        };

        self.data.drain(range.clone());

        self.move_highlights(range.clone(), true);

        let current = match &self.selection {
            Selection::Single(current) => *current,
            Selection::Visual { range, .. } => range.start,
        };

        if self.data.is_empty() {
            self.data.push(0);
        }

        let current = current.min(self.data.len() - 1);

        self.selection = Selection::Single(current);
        self.set_mode(Mode::Normal);
    }
}

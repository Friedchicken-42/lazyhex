use std::fs::read;

use mlua::{Function, Lua, Table};
type Result<T> = std::result::Result<T, mlua::Error>;

#[derive(Debug, Clone, Copy)]
pub enum Endian {
    Little,
    Big,
}

#[derive(Debug, Clone, Copy)]
pub enum HighlightUpdate {
    Update,
    Reload,
}

#[derive(Debug)]
pub struct Config<'lua> {
    pub page: i32,
    pub endian: Endian,
    pub highlight: Function<'lua>,
    pub empty_value: u8,
    pub on_delete: HighlightUpdate,
}

impl<'lua> Config<'lua> {
    fn default(lua: &'lua Lua) -> Self {
        let highlight = lua.create_function(|_, _: ()| Ok(())).unwrap();

        Self {
            page: 256,
            endian: Endian::Big,
            highlight,
            empty_value: 0x00,
            on_delete: HighlightUpdate::Reload,
        }
    }

    pub fn load(lua: &'lua Lua) -> Result<Self> {
        let config = Self::default(lua);

        let table = if let Ok(config) = read("config.lua") {
            lua.load(config).eval::<Table>()
        } else if let Ok(config) = read("~/.config/lzh/config.lua") {
            lua.load(config).eval::<Table>()
        } else {
            lua.create_table()
        }?;

        let endian = match table.get::<&str, String>("endian") {
            Ok(s) if s == "little" || s == "l" => Endian::Little,
            Ok(s) if s == "big" || s == "b" => Endian::Big,
            Ok(s) => panic!("wrong endian type: `{s}`"),
            Err(_) => config.endian,
        };

        let on_delete = match table.get::<&str, String>("on_delete") {
            Ok(s) if s == "update" => HighlightUpdate::Update,
            Ok(s) if s == "reload" => HighlightUpdate::Reload,
            Ok(s) => panic!("`{s}` can only be `delete | reload`"),
            Err(_) => config.on_delete,
        };

        Ok(Self {
            page: table.get("page").unwrap_or(config.page),
            endian,
            highlight: table.get("highlight").unwrap_or(config.highlight),
            empty_value: table.get("empty_value").unwrap_or(config.empty_value),
            on_delete,
        })
    }
}

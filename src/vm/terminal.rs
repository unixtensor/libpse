use mlua::{UserDataFields, Lua as Luau, Result as lResult, Table, UserData};
use const_format::str_split;
use crossterm::style::Stylize;

use crate::session::Pse;

macro_rules! foreground_styles_luau {
	($luau:expr, $style_table:expr, $($color:ident)+) => {
		$(
			$style_table.raw_set(stringify!($color).to_ascii_uppercase(), $luau.create_function(|_, text: String| -> lResult<String> {
            	Ok(text.$color().to_string())
        	})?)?;
        )+
    };
}
macro_rules! background_styles_luau {
	($luau:expr, $style_table:expr, $($color:ident)+) => {
		$(
			$style_table.raw_set(
				str_split!(stringify!($color), "_")[1..].join("_").to_ascii_uppercase(),
			$luau.create_function(|_, text: String| -> lResult<String> {
				Ok(text.$color().to_string())
        	})?)?;
        )+
    };
}

fn text_styles_funcs(luau: &Luau) -> lResult<(Table, Table)> {
	let foreground_table = luau.create_table()?;
	let background_table = luau.create_table()?;
	foreground_styles_luau!(luau, foreground_table,
		dark_grey   dark_red     dark_green dark_cyan
		dark_yellow dark_magenta dark_blue
		red  grey    black green yellow
	    blue magenta cyan  white
		underlined
		underline_dark_grey   underline_dark_red     underline_dark_green underline_dark_cyan
		underline_dark_yellow underline_dark_magenta underline_dark_blue  underline_red
		underline_grey        underline_black        underline_green      underline_yellow
		underline_blue        underline_magenta      underline_cyan       underline_white
		bold
		crossed_out
	);
	background_styles_luau!(luau, background_table,
		on_dark_grey   on_dark_red     on_dark_green on_dark_cyan
		on_dark_yellow on_dark_magenta on_dark_blue
		on_red   on_grey  on_black
	    on_green on_yellow
	    on_blue  on_magenta
	    on_cyan  on_white
	);
	Ok((foreground_table, background_table))
}

fn fields_write_funcs<F: UserDataFields<Terminal>>(fields: &mut F) {
	fields.add_field_method_get("WRITE", |luau, _| luau.create_function(|_, s: String| -> lResult<()> {
		print!("{s}");
		Ok(())
	}));
	fields.add_field_method_get("WRITE_ERROR", |luau, _| luau.create_function(|_, s: String| -> lResult<()> {
		eprint!("{s}");
		Ok(())
	}));
	fields.add_field_method_get("WRITE_ERROR_LN", |luau, _| luau.create_function(|_, s: String| -> lResult<()> {
		eprintln!("{s}");
		Ok(())
	}));
}

struct Terminal;
impl UserData for Terminal {
	fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
		fields_write_funcs(fields);
		fields.add_field_method_get("OUT", |luau, _| {
			let (foreground, background) = text_styles_funcs(luau)?;
			let out_table = luau.create_table()?;
			out_table.raw_set("FOREGROUND", foreground)?;
			out_table.raw_set("BACKGROUND", background)?;
			Ok(out_table)
		});
	}
	// fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {

	// }
}

pub trait TerminalGlobal {
	fn vm_glob_terminal(&self, luau_globals: &Table) -> lResult<()>;
}
impl TerminalGlobal for Pse {
	fn vm_glob_terminal(&self, luau_globals: &Table) -> lResult<()> {
		luau_globals.raw_set("TERMINAL", Terminal)
	}
}
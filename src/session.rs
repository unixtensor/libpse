use mlua::Lua as Luau;
use std::{cell::RefCell, fs, rc::Rc, usize};
use core::fmt;

use crate::{
	history::History, rc::{self}, terminal::TermProcessor, vm::LuauVm
};

pub trait MapDisplay<T, E: fmt::Display> {
	fn map_or_display<F: FnOnce(T)>(self, f: F);
	fn map_or_display_none<R, F: FnOnce(T) -> Option<R>>(self, f: F) -> Option<R>;
}
impl<T, E: fmt::Display> MapDisplay<T, E> for Result<T, E> {
	///Map but display the error to stdout
	#[inline]
	fn map_or_display<F: FnOnce(T)>(self, f: F) {
		self.map_or_else(|e| shell_error(e), f)
	}
	///Map but display the error to stdout and return `None`
	#[inline]
	fn map_or_display_none<R, F: FnOnce(T) -> Option<R>>(self, f: F) -> Option<R> {
		self.map_or_else(|e| { shell_error(e); None }, f)
	}
}

pub fn shell_error<E: fmt::Display>(err: E) {
	color_print::ceprintln!("<bold,r>[!]:</> {err}")
}
pub fn shell_error_none<T, E: fmt::Display>(err: E) -> Option<T> {
	shell_error(err);
	None
}

#[derive(Debug, Clone)]
pub struct VmConfig {
	pub sandbox: bool,
	pub jit: bool,
}
#[derive(Debug, Clone)]
pub struct Config {
	pub norc: bool,
	pub vm: VmConfig,
}
#[derive(Debug, Clone)]
pub struct Input {
	pub literal: String,
	pub cursor: usize,
}
pub struct Rt {
	pub input: Input,
	pub ps: Rc<RefCell<String>>,
	pub vm: Luau,
}
pub struct Pse {
	pub config: Config,
	pub history: History,
	pub rt: Rt
}
impl Pse {
	const DEFAULT_PS: &str = concat!("pse-", env!("CARGO_PKG_VERSION"), "$ ");

	pub fn create(config: Config) -> Self {
		let rt = Rt {
			ps: Rc::new(RefCell::new(Self::DEFAULT_PS.to_owned())),
			vm: Luau::new(),
			input: Input {
				literal: String::new(),
				cursor: usize::MIN,
			},
		};

		Self { rt, config, history: History::init() }
	}

	pub fn start(&mut self) {
		if !self.config.norc {
			if let Some(conf_file) = rc::config_file() {
				fs::read_to_string(conf_file).map_or_display(|luau_conf| self.vm_exec(luau_conf));
			}
		};
		self.term_input_processor().map_or_display(|()| self.history.write_to_file_fallible())
	}
}

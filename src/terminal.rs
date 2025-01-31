use crossterm::{cursor, event::{self, Event, KeyCode, KeyEvent, KeyModifiers}, execute, terminal};
use core::fmt;
use std::io::{self, Write};
use thiserror::Error;

use crate::{commands::Command, session::{self, Pse}};

#[derive(Debug, Error)]
pub enum InputHandleError {
	#[error("UserExit")]
	UserExit,
	#[error("Sigint")]
	Sigint,
	#[error("Render failure: {0}")]
	Write(io::Error),
	#[error("Flush failure: {0}")]
	Flush(io::Error),
	#[error("Disabling the terminal's raw mode failed: {0}")]
	EnableRaw(io::Error),
	#[error("Enabling the terminal's raw mode failed: {0}")]
	DisableRaw(io::Error),
	#[error("key input failure: {0}")]
	Key(KeyCode),
}
type InputResult<T> = Result<T, InputHandleError>;

#[allow(dead_code)]
fn debug<S: fmt::Display>(s: S) {
	terminal::disable_raw_mode().unwrap();
	println!("{s}");
	terminal::enable_raw_mode().unwrap()
}

trait SpecificKeybinds {
	const EXIT_ID_1: &str;
	const KEY_SPACE: char;
	fn key_literal(&mut self, keycode: KeyCode) -> InputResult<()>;
	fn key_ctrl(&mut self, input_key: KeyEvent, keycode: KeyCode) -> InputResult<()>;
	fn key_enter(&mut self) -> InputResult<()>;
	fn key_backspace(&mut self) -> InputResult<()>;
	fn key_arrow_right(&mut self) -> InputResult<()>;
	fn key_arrow_left(&mut self) -> InputResult<()>;
	fn key_arrow_up(&mut self) -> InputResult<()>;
	fn key_arrow_down(&mut self) -> InputResult<()>;
}
impl SpecificKeybinds for Pse {
	const EXIT_ID_1: &str = "exit";
	const KEY_SPACE: char = ' ';

	fn key_literal(&mut self, keycode: KeyCode) -> InputResult<()> {
		match keycode {
			KeyCode::Char(Self::KEY_SPACE) => self.term_render(Self::KEY_SPACE.to_string()),
			keycode => self.term_render(keycode.to_string())
		}
	}

	fn key_ctrl(&mut self, input_key: KeyEvent, keycode: KeyCode) -> InputResult<()> {
		match input_key.modifiers.contains(KeyModifiers::CONTROL) {
			true => match keycode {
				KeyCode::Char('c') => Err(InputHandleError::Sigint),
				KeyCode::Char('r') => unimplemented!(),
				_ => Ok(())
			},
			false => self.key_literal(keycode)
		}
	}

	fn key_enter(&mut self) -> InputResult<()> {
		if self.rt.input.literal == Self::EXIT_ID_1 { return Err(InputHandleError::UserExit) };

		terminal::disable_raw_mode().map_err(InputHandleError::DisableRaw)?;
		println!();
		self.spawn_sys_cmd();
		self.rt.input.literal.clear();
		self.rt.input.cursor = u16::MIN;
		self.term_render_ps()
	}

	fn key_backspace(&mut self) -> InputResult<()> {
		if self.rt.input.literal.pop().is_some() {
			execute!(
				io::stdout(),
				cursor::MoveLeft(1),
				terminal::Clear(terminal::ClearType::UntilNewLine)
			).map_err(InputHandleError::Flush)?;
			self.rt.input.cursor-=1;
		}
		Ok(())
	}

	fn key_arrow_right(&mut self) -> InputResult<()> {
		match self.term_input_cursor_move_right() {
			Some(()) => execute!(io::stdout(), cursor::MoveRight(1)).map_err(InputHandleError::Flush),
			None => Ok(())
		}
	}

	fn key_arrow_left(&mut self) -> InputResult<()> {
		match self.term_input_cursor_move_left() {
			Some(()) => execute!(io::stdout(), cursor::MoveLeft(1)).map_err(InputHandleError::Flush),
			None => Ok(())
		}
	}

	fn key_arrow_up(&mut self) -> InputResult<()> {
		self.rt.history.index += 1;
		if self.rt.history.index != self.rt.history.history.len() as isize {
			self.term_render_reset()?;
			self.term_render(self.rt.history.history[self.rt.history.index as usize].clone())?;
		}
		Ok(())
	}

	fn key_arrow_down(&mut self) -> InputResult<()> {
		self.rt.history.index -= 1;
		if self.rt.history.index != -1 {
			self.term_render_reset()?;
			self.term_render(self.rt.history.history[self.rt.history.index as usize].clone())?;
		}
		Ok(())
	}
}

pub trait TermInputCursor {
	fn term_input_cursor_move_left(&mut self) -> Option<()>;
	fn term_input_cursor_move_right(&mut self) -> Option<()>;
}
impl TermInputCursor for Pse {
	fn term_input_cursor_move_left(&mut self) -> Option<()> {
		if self.rt.input.cursor == u16::MIN { None } else {
			match self.rt.input.cursor>u16::MIN {
				true => { self.rt.input.cursor-=1; Some(()) }
				false => None
			}
		}
	}

	fn term_input_cursor_move_right(&mut self) -> Option<()> {
		if self.rt.input.cursor == u16::MAX { None } else {
			match self.rt.input.cursor<self.rt.input.literal.chars().count() as u16 {
				true => { self.rt.input.cursor+=1; Some(()) },
				false => None
			}
		}
	}
}

pub trait TermProcessor {
	fn term_render(&mut self, def_string: String) -> InputResult<()>;
	fn term_render_ps(&self) -> InputResult<()>;
	fn term_render_reset(&mut self) -> InputResult<()>;
	fn term_input_handler(&mut self, input_key: KeyEvent) -> Option<()>;
	fn term_input_mainthread(&mut self) -> io::Result<()>;
	fn term_input_processor(&mut self) -> io::Result<()>;
}
impl TermProcessor for Pse {
	fn term_render(&mut self, text: String) -> InputResult<()> {
		self.rt.input.literal.insert_str(self.rt.input.cursor.into(), &text);
		let input_literal_size = self.rt.input.literal.chars().count() as u16;
		self.rt.input.cursor = input_literal_size;
		if self.rt.input.cursor != input_literal_size {
			execute!(io::stdout(), terminal::Clear(terminal::ClearType::UntilNewLine)).map_err(InputHandleError::Flush)?;
			let slice = &self.rt.input.literal[(self.rt.input.cursor as usize)..];
			write!(io::stdout(), "{text}{slice}").map_err(InputHandleError::Write)?;
		} else {
			write!(io::stdout(), "{text}").map_err(InputHandleError::Write)?;
		}
		io::stdout().flush().map_err(InputHandleError::Flush)
	}

	fn term_render_ps(&self) -> InputResult<()> {
		print!("{}", self.rt.ps.borrow());
		io::stdout().flush().map_err(InputHandleError::Flush)
	}

	fn term_render_reset(&mut self) -> InputResult<()> {
		execute!(io::stdout(), cursor::MoveLeft(self.rt.input.cursor)).map_err(InputHandleError::Flush)?;
		self.rt.input.cursor = u16::MIN;
		self.rt.input.literal.clear();
		execute!(io::stdout(), terminal::Clear(terminal::ClearType::UntilNewLine)).map_err(InputHandleError::Flush)
	}

	fn term_input_handler(&mut self, input_key: KeyEvent) -> Option<()> {
		let input_handle = match input_key.code {
			KeyCode::Enter     => self.key_enter(),
			KeyCode::Backspace => self.key_backspace(),
			KeyCode::Tab       => todo!(),
			KeyCode::Right     => self.key_arrow_right(),
			KeyCode::Left      => self.key_arrow_left(),
			KeyCode::Up        => self.key_arrow_up(),
			KeyCode::Down      => self.key_arrow_down(),
			keycode            => self.key_ctrl(input_key, keycode)
		};
		input_handle.map_or_else(|inp_err| match inp_err {
			InputHandleError::UserExit => None,
		    InputHandleError::Sigint => self.term_render("^C".to_owned()).ok(),
			input_err => session::shell_error_none(input_err)
		}, Some)
	}

	fn term_input_mainthread(&mut self) -> io::Result<()> {
		execute!(io::stdout(), event::EnableBracketedPaste)?;
		self.term_render_ps().map_or_else(|_| Ok(()), |()| {
			loop {
				terminal::enable_raw_mode()?;
			    if let Event::Key(event) = event::read()? {
					if self.term_input_handler(event).is_none() { break Ok(()) }
				}
			}
		})
	}

	fn term_input_processor(&mut self) -> io::Result<()> {
		self.term_input_mainthread()?;
	    terminal::disable_raw_mode()?;
	    execute!(io::stdout(), event::DisableBracketedPaste)
	}
}
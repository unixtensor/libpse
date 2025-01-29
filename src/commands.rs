use std::{env, path::{Path, PathBuf}, process, str::SplitWhitespace};
use uzers::User;

use crate::{session::{MapDisplay, Pse}, valid_pbuf::IsValid};

trait PathBufIsValid {
	fn is_valid_or_home(&self) -> Option<PathBuf>;
}
impl PathBufIsValid for PathBuf {
	fn is_valid_or_home(&self) -> Option<PathBuf> {
		self.is_valid_or(self.is_dir(), home::home_dir)
	}
}

struct ChangeDirectory(Option<PathBuf>);
impl ChangeDirectory {
	fn set_dir(&mut self, new_path: &Path) -> Option<PathBuf> {
		let past_dir = env::current_dir().ok();
		env::set_current_dir(new_path).map_or_display_none(|()| {
			self.0 = past_dir;
			Some(new_path.to_path_buf())
		})
	}

	fn home_dir(&mut self) -> Option<PathBuf> {
		home::home_dir().map_or(self.set_dir(Path::new("/")), |home_pathbuf| self.set_dir(&home_pathbuf))
	}

	fn previous_dir(&mut self) -> Option<PathBuf> {
		match self.0.as_ref() {
			Some(p_buf) => self.set_dir(&p_buf.to_path_buf()),
			None => None
		}
	}

	fn specific_user_dir(&self, requested_user: String) -> Option<PathBuf> {
		match requested_user.as_str() {
			"root" => PathBuf::from("/root").is_valid_or_home(),
			u => {
				for user in unsafe { uzers::all_users().collect::<Vec<User>>() } {
					let user_name = user.name();
					if *u == *user_name {
						let mut user_dir = PathBuf::from("/home");
						user_dir.push(user_name);
						return user_dir.is_valid_or_home();
					}
				}
				None
			}
		}
	}

	fn cd_args(&mut self, vec_args: Vec<String>) -> Option<PathBuf> {
		let string_path = vec_args.concat();
		let new_path = Path::new(string_path.as_str());
		match new_path.is_dir() {
			true => self.set_dir(new_path),
			false => {
				match new_path.file_name() {
					Some(file_name) => println!("cd: {:?} is not a directory.", file_name),
					None => println!("cd: Failed to resolve the file name of a file that is not a directory."),
				}
				None
			}
		}
	}

	fn change_directory(&mut self, args: SplitWhitespace) -> Option<PathBuf> {
		let vec_args: Vec<String> = args.map(|arg| arg.to_owned()).collect();
		match vec_args.first() {
			None => self.home_dir(),
			Some(arg) => match arg.as_str() {
				"/" => self.set_dir(Path::new("/")),
				"-" => self.previous_dir(),
				arg_str => {
					let mut arg_chars = arg_str.chars();
					match arg_chars.next() {
						Some(char) => match char == '~' {
							true => self.specific_user_dir(arg_chars.collect::<String>()),
							false => self.cd_args(vec_args),
						},
						None => self.home_dir(),
					}
				}
			},
		}
	}
}

pub trait Command {
	fn spawn_sys_cmd(&mut self);
}
impl Command for Pse {
	fn spawn_sys_cmd(&mut self) {
		let mut args = self.rt.input.literal.split_whitespace();
		if let Some(command) = args.next() {
			match command {
				"cd" => if ChangeDirectory(None).change_directory(args).is_some() {
					self.history.add(self.rt.input.literal.as_str())
				},
				command => if let Ok(mut child) = process::Command::new(command).args(args).spawn() {
	    			self.history.add(self.rt.input.literal.as_str());
					child.wait().ok();
				} else {
		   			println!("pse: Unknown command: {}", self.rt.input.literal)
				}
			}
		}
	}
}

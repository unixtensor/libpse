use std::{process, str::SplitWhitespace, path::{Path, PathBuf}};
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

struct ChangeDirectory;
impl ChangeDirectory {
	fn set_current_dir(&self, new_path: &Path) -> Option<PathBuf> {
		std::env::set_current_dir(new_path).map_or_display_none(|()| Some(new_path.to_path_buf()))
	}

	fn home_dir(&self) -> Option<PathBuf> {
		home::home_dir().map_or(self.set_current_dir(Path::new("/")), |home_pathbuf| self.set_current_dir(&home_pathbuf))
	}

	fn previous_dir(&self) -> Option<PathBuf> {
		unimplemented!()
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

	fn cd_args(&self, vec_args: Vec<String>) -> Option<PathBuf> {
		let string_path = vec_args.concat();
		let new_path = Path::new(string_path.as_str());
		match new_path.is_dir() {
			true => self.set_current_dir(new_path),
			false => {
				match new_path.file_name() {
					Some(file_name) => println!("cd: {:?} is not a directory.", file_name),
					None => println!("cd: Failed to resolve the file name of a file that is not a directory."),
				}
				None
			}
		}
	}

	fn change_directory(&self, args: SplitWhitespace) -> Option<PathBuf> {
		let vec_args: Vec<String> = args.map(|arg| arg.to_owned()).collect();
		match vec_args.first() {
			None => self.home_dir(),
			Some(arg) => match arg.as_str() {
				"/" => self.set_current_dir(Path::new("/")),
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
		let mut args = self.rt.input.split_whitespace();
		if let Some(command) = args.next() {
			match command {
				"cd" => if ChangeDirectory.change_directory(args).is_some() { self.history.add(self.rt.input.as_str()) },
				command => if let Ok(mut child) = process::Command::new(command).args(args).spawn() {
	    			self.history.add(self.rt.input.as_str());
					child.wait().ok();
				} else {
		   			println!("\npse: Unknown command: {}", self.rt.input)
				}
			}
		}
	}
}

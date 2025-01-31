use std::{fs::{File, OpenOptions}, io::{BufRead, BufReader, Write}, path::PathBuf};

use crate::{rc::{self}, session::{self, MapDisplay}, valid_pbuf::IsValid};

#[derive(Debug, Clone)]
pub struct History {
	pub history: Vec<String>,
	pub index: isize,
	file: Option<PathBuf>,
}
impl History {
	pub fn init() -> Self {
		let mut history = Vec::new();
		let file = rc::config_dir().map(|mut config_dir| {
			config_dir.push(".history");
			config_dir.is_valid_file_or_create(b"");
			config_dir
		});
		file.as_ref().and_then(|file| {
			File::open(file).map_or_display_none(|file| {
				let mut fs_history_vec = BufReader::new(file).lines().map_while(Result::ok).collect::<Vec<String>>();
				fs_history_vec.dedup();
				fs_history_vec.reverse();
				Some(fs_history_vec)
			})
		}).inspect(|fs_history_vec| history = fs_history_vec.clone());

		Self { history, file, index: -1 }
	}

	pub fn write_to_file_fallible(&mut self) {
		if self.history.is_empty() { return; }

		if let Some(history_file) = &self.file {
			OpenOptions::new()
				.append(true)
				.open(history_file.as_path())
			.map_or_display(|mut file| {
				let newline_maybe = if self.history.is_empty() { "" } else { "\n" };
				let formatted = format!("{newline_maybe}{}", self.history.join("\n"));
				file.write_all(formatted.as_bytes()).unwrap_or_else(session::shell_error)
			});
		}
	}

	pub fn add(&mut self, command: &str) {
		match self.history.last() {
		    Some(last_cmd) => if last_cmd != command { self.history.push(command.to_owned()); },
		    None => self.history.push(command.to_owned()),
		};
	}
}
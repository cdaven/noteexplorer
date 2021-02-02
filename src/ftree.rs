use ansi_term::Colour;
use std::path;
use walkdir::{DirEntry, WalkDir};

pub fn get_files(root: &path::Path, ext: &str) -> Vec<path::PathBuf> {
	let mut files = Vec::new();

	if root.is_dir() {
		let walker = WalkDir::new(root).into_iter();
		for entry in walker.filter_entry(|e| !is_hidden(e)) {
			let entry = match entry {
				Ok(e) => e,
				Err(err) => {
					let path = err.path().unwrap_or_else(|| path::Path::new("")).display();
					eprintln!(
						"{} Couldn't access {}",
						Colour::Yellow.paint("Warning:"),
						path
					);
					continue;
				}
			};

			let path = entry.into_path();
			if ext == path.extension().unwrap_or_default().to_str().unwrap() {
				files.push(path);
			}
		}
	}

	files
}

fn is_hidden(entry: &DirEntry) -> bool {
	entry
		.file_name()
		.to_str()
		.map(|s| s.starts_with('.'))
		.unwrap_or(false)
}

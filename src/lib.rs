mod ftree;
mod mdparse;
mod note;

use chrono::Utc;
use debug_print::debug_println;
use note::{NoteCollection, NoteMeta};
use std::error::Error;
use std::fs;

#[derive(Debug)]
pub struct Config {
	pub id_pattern: String,
	pub backlinks_heading: String,
	pub extension: String,
	pub path: String,
	pub command: String,
	pub force: bool,
}

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
	let start_time = Utc::now();
	let notes = NoteCollection::collect_files(
		&fs::canonicalize(&config.path)?,
		&config.extension,
		mdparse::NoteParser::new(&config.id_pattern, &config.backlinks_heading)?,
	);
	let duration_collect_files = Utc::now() - start_time;

	let start_time = Utc::now();
	match config.command.as_str() {
		"list-broken-links" => print_broken_links(&notes),
		"list-sources" => print_sources(&notes),
		"list-sinks" => print_sinks(&notes),
		"list-isolated" => print_isolated(&notes),
		"list-tasks" => print_tasks(&notes),
		"remove-backlinks" => remove_backlinks(&notes),
		"update-backlinks" => update_backlinks(&notes),
		"update-filenames" => update_filenames(&notes, config.force)?,
		_ => print_stats(&notes),
	}
	let duration_subcommand = Utc::now() - start_time;

	debug_println!(
		"NoteCollection::collect_files() took {} ms",
		duration_collect_files.num_milliseconds()
	);
	debug_println!(
		"Subcommand {} took {} ms",
		&config.command,
		duration_subcommand.num_milliseconds()
	);

	Ok(())
}

fn print_stats(note_collection: &NoteCollection) {
	println!("# Statistics\n");

	println!("- Notes in collection: {}", note_collection.count());
	println!("- Notes with ID: {}", note_collection.count_with_id());
	println!("- Wikilinks: {}", note_collection.count_links());
}

fn print_tasks(note_collection: &NoteCollection) {
	let tasks = note_collection.get_tasks();
	let num_tasks: usize = tasks.iter().map(|(_, t)| t.len()).sum();
	println!("# Tasks\n");
	println!("There are {} tasks in your notes", num_tasks);

	for (note, note_tasks) in tasks {
		println!("\n## {}\n", note.get_wikilink_to());

		for task in note_tasks {
			println!("- [ ] {}", task);
		}
	}
}

fn print_sources(note_collection: &NoteCollection) {
	let notes = note_collection.get_sources();

	println!("# Source notes\n");
	println!(
		"{} notes have no incoming links, but at least one outgoing link\n",
		notes.len()
	);
	print_note_wikilink_list(&notes);
}

fn print_sinks(note_collection: &NoteCollection) {
	let notes = note_collection.get_sinks();

	println!("# Sink notes\n");
	println!(
		"{} notes have no outgoing links, but at least one incoming link\n",
		notes.len()
	);
	print_note_wikilink_list(&notes);
}

fn print_isolated(note_collection: &NoteCollection) {
	let notes = note_collection.get_isolated();

	println!("# Isolated notes\n");
	println!("{} notes have no incoming or outgoing links\n", notes.len());
	print_note_wikilink_list(&notes);
}

fn print_note_wikilink_list(notes: &[NoteMeta]) {
	for note in notes {
		println!("- {}", note.get_wikilink_to());
	}
}

fn print_broken_links(note_collection: &NoteCollection) {
	let broken_links = note_collection.get_broken_links();

	println!("# Broken links\n");

	for (link, notes) in broken_links {
		let linkers: Vec<String> = notes.iter().map(|n| n.get_wikilink_to()).collect();
		println!("- \"{}\" links to unknown {}", linkers.join(" and "), link);
	}
}

fn remove_backlinks(note_collection: &NoteCollection) {
	let removed = note_collection.remove_backlinks();
	println!("Removed backlinks section from {} notes", removed.len());
}

fn update_backlinks(note_collection: &NoteCollection) {
	let updated = note_collection.update_backlinks();
	println!("Updated backlinks section in {} notes", updated.len());

	for note in updated {
		println!("- {}", note.get_wikilink_to());
	}
}

fn update_filenames(note_collection: &NoteCollection, force: bool) -> Result<(), Box<dyn Error>> {
	let mut affected_backlinks = false;
	for (note, new_stem) in note_collection.get_mismatched_filenames() {
		let original_filename = format!("{}.{}", note.stem, note.extension);
		let new_filename = format!("{}.{}", new_stem, note.extension);
		let reply = if force {
			"y".to_owned()
		} else {
			rprompt::prompt_reply_stdout(&format!(
				"Rename \"{}\" to \"{}\"? ([y]/n) ",
				original_filename, new_filename
			))?
		};

		if reply == "y" || reply.is_empty() {
			if note.path.ends_with(&original_filename) {
				// If note has links to other notes and NOT an ID, this means that there
				// are backlinks in other notes that are now linking to the old filename
				affected_backlinks = affected_backlinks || (note.has_links && note.id.is_none());

				let updated_notes = note_collection.rename_note(&note, &new_stem)?;
				if updated_notes.len() > 0 {
					for n in updated_notes {
						println!("- Updated link from {}", n.get_wikilink_to());
					}
				}
			} else {
				// TODO: Return as Err
				eprintln!("Error: probably a bug in how the file name path is determined");
			}
		}
	}

	if affected_backlinks {
		println!("You should probably update-backlinks now");
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use crate::*;
	use std::env::temp_dir;
	use std::io::Write;
	use std::path::PathBuf;
	use std::{fs, io};

	/// Create directory, removing it first if it exists,
	/// together with all files and subdirectories. Careful!
	fn create_dir(dir: &PathBuf) -> io::Result<()> {
		if dir.exists() {
			fs::remove_dir_all(dir)?;
		}

		fs::create_dir(&dir)?;

		Ok(())
	}

	fn write_to_tmp_file(dir: &mut PathBuf, filename: &str, contents: &str) -> io::Result<()> {
		dir.push(filename);
		let mut file = fs::File::create(dir)?;
		file.write_all(contents.as_bytes())?;
		Ok(())
	}

	#[test]
	fn rename_file() {
		let mut dir = temp_dir();
		dir.push("noteexplorer-test-rename");
		create_dir(&dir).unwrap();

		write_to_tmp_file(&mut dir.clone(), "noteexplorer-test-rename-1.md", "# Rename This 1\r\nHere is a link to another file: [[Noteexplorer-test-rename-2]]. And some text after. [[Backlink-File]]").unwrap();
		write_to_tmp_file(&mut dir.clone(), "noteexplorer-test-rename-2.md", "# Rename Then 2\r\nHere is a link to another file: [[noteexplorer-TEST-rename-1]]. And some text after. [[Backlink-File]]").unwrap();
		write_to_tmp_file(&mut dir.clone(), "noteexplorer-test-rename-3.md", "# Rename That 3\r\nHere are the links: [[noteexplorer-test-rename-1]] and [[noteexplorer-test-rename-2]]. And some text after").unwrap();
		write_to_tmp_file(
			&mut dir.clone(),
			"backlink-file.md",
			"## Backlinks\n\n- [[noteexplorer-TEST-rename-1]]\n\n- [[noteexplorer-TEST-rename-2]]",
		)
		.unwrap();

		let notes_before = NoteCollection::collect_files(
			&dir,
			&"md",
			crate::mdparse::NoteParser::new(&r"\d{14}", &"## Backlinks").unwrap(),
		);

		// No extra notes should be found
		assert_eq!(notes_before.count(), 4);
		// No broken links in the test data
		assert_eq!(notes_before.get_broken_links().len(), 0);

		update_filenames(&notes_before, true).unwrap();

		let notes_after = NoteCollection::collect_files(
			&dir,
			&"md",
			crate::mdparse::NoteParser::new(&r"\d{14}", &"## Backlinks").unwrap(),
		);

		for note in notes_after.into_meta_vec() {
			match note.title.as_str() {
				"Rename This 1" => {
					assert_eq!(note.stem, "Rename This 1");
				}
				"Rename Then 2" => {
					assert_eq!(note.stem, "Rename Then 2");
				}
				"Rename That 3" => {
					assert_eq!(note.stem, "Rename That 3");
				}
				"backlink-file" => {
					assert_eq!(note.stem, "backlink-file");
				}
				_ => {
					panic!("Unrecognized note title");
				}
			};
		}

		assert_eq!(notes_after.count(), 4);
		assert_eq!(notes_after.get_broken_links().len(), 0);
		assert_eq!(notes_after.get_isolated().len(), 0);
		assert_eq!(notes_after.get_sources().len(), 1);
		assert_eq!(notes_after.get_sinks().len(), 1);
	}
}

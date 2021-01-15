use ansi_term::{Colour, Style};
use regex::Regex;
use std::collections::HashMap;
use std::collections::HashSet;
use std::error::Error;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::{fs, io, path};

#[derive(Debug)]
struct NoteFile {
	path: String,
	stem: String,
	extension: String,
	content: String,
}

impl NoteFile {
	fn new(path: &path::PathBuf) -> NoteFile {
		NoteFile {
			path: path.as_os_str().to_str().unwrap().to_string(),
			stem: path
				.file_stem()
				.expect("Error in file_stem()")
				.to_str()
				.unwrap()
				.to_string(),
			extension: path
				.extension()
				.expect("Error in extension()")
				.to_str()
				.unwrap()
				.to_string(),
			content: fs::read_to_string(&path).expect("Error in read_to_string()"),
		}
	}

	fn escape_filename(filename: &str) -> String {
		// TODO: Make regexes static somehow
		// These characters are replaced with " " (illegal in Windows)
		let illegal_chars = Regex::new("[<>:*?/\"\\\\]").unwrap();
		// "." at the beginning or end are removed
		let surrounding_stops = Regex::new(r"(^\.|\.$)").unwrap();
		// Replace double spaces with single
		let double_spaces = Regex::new(r" +").unwrap();

		double_spaces
			.replace_all(
				&surrounding_stops
					.replace_all(&illegal_chars.replace_all(&filename, " ").to_string(), "")
					.to_string(),
				" ",
			)
			.trim()
			.to_string()
	}

	fn begins_or_ends_with_dot_or_space(filename: &str) -> bool {
		if filename.len() == 0 {
			return false;
		}
		let first_char = filename.chars().nth(0).unwrap();
		let last_char = filename.chars().last().unwrap();
		first_char == ' ' || first_char == '.' || last_char == ' ' || last_char == '.'
	}
}

#[derive(Debug)]
struct Note {
	file: NoteFile,
	title: String,
	id: Option<String>,
	links: HashSet<WikiLink>,
	todos: Vec<String>,
	parser: Rc<NoteParser>,
}

// Use path as unique identifier for notes
impl PartialEq for Note {
	fn eq(&self, other: &Self) -> bool {
		self.file.path == other.file.path
	}
}

impl Eq for Note {}

// Use path as unique identifier for notes
impl Hash for Note {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.file.path.hash(state);
	}
}

impl Note {
	fn new(file: NoteFile, parser: Rc<NoteParser>) -> Note {
		Note {
			id: Note::get_id_assoc(&file, &parser),
			title: Note::get_title_assoc(&file, &parser),
			links: Note::get_links_assoc(&file, &parser),
			todos: Note::get_todos_assoc(&file, &parser),
			parser,
			file,
		}
	}

	fn get_id_assoc(file: &NoteFile, parser: &NoteParser) -> Option<String> {
		// First try to get ID from note text
		if let Some(id) = parser.get_id(&file.content) {
			Some(id)
		}
		// Then try to get ID from note file name
		else if let Some(id) = parser.get_id(&file.stem) {
			Some(id)
		} else {
			None
		}
	}

	fn get_title_assoc(file: &NoteFile, parser: &NoteParser) -> String {
		// Try first with a H1 title in the contents
		if let Some(title) = parser.get_h1(&file.content) {
			title
		} else {
			// Then use file stem without ID (even if the wrong ID)
			parser.remove_id(&file.stem).trim().to_string()
		}
	}

	fn get_links_assoc(file: &NoteFile, parser: &NoteParser) -> HashSet<WikiLink> {
		parser.get_wiki_links(parser.get_content_without_backlinks(&file.content))
	}

	fn get_todos_assoc(file: &NoteFile, parser: &NoteParser) -> Vec<String> {
		parser.get_todos(&file.content)
	}

	/** Return a copy of the note's meta data */
	fn get_meta(&self) -> NoteMeta {
		NoteMeta {
			path: self.file.path.clone(),
			stem: self.file.stem.clone(),
			extension: self.file.extension.clone(),
			title: self.title.clone(),
			id: self.id.clone(),
		}
	}
}

#[derive(PartialEq, Eq, Hash, Debug)]
struct NoteMeta {
	path: String,
	stem: String,
	extension: String,
	title: String,
	id: Option<String>,
}

impl NoteMeta {
	fn get_wikilink_to(&self) -> String {
		let id = self.id.as_ref().unwrap_or(&self.stem);
		let title = if &self.title == id {
			None
		} else {
			Some(&self.title)
		};

		// When the id and title are the same, don't repeat the title
		format!("[[{}]] {}", id, title.unwrap_or(&String::from("")))
			.trim_end()
			.to_string()
	}
}

#[derive(Eq, Clone, Debug)]
enum WikiLink {
	Id(String),
	FileName(String),
}

// Case-insensitive matching for the WikiLink value
impl PartialEq for WikiLink {
	fn eq(&self, other: &Self) -> bool {
		use WikiLink::*;
		match (self, other) {
			(Id(a), Id(b)) => a.to_lowercase() == b.to_lowercase(),
			(FileName(a), FileName(b)) => a.to_lowercase() == b.to_lowercase(),
			_ => false,
		}
	}
}

// Case-insensitive hashing for the WikiLink value
impl Hash for WikiLink {
	fn hash<H: Hasher>(&self, state: &mut H) {
		use WikiLink::*;
		match self {
			Id(link) => link.to_lowercase().hash(state),
			FileName(link) => link.to_lowercase().hash(state),
		}
	}
}

impl fmt::Display for WikiLink {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		use WikiLink::*;
		match self {
			Id(link) => write!(f, "[[{}]]", link),
			FileName(link) => write!(f, "[[{}]]", link),
		}
	}
}

#[derive(Debug)]
struct NoteParser {
	id_pattern: String,
	id_expr: Regex,
	wiki_link_expr: Regex,
	h1_expr: Regex,
	todo_expr: Regex,
	backlinks_heading: String,
	backlink_expr: Regex,
}

#[allow(dead_code)]
impl NoteParser {
	fn new(id_pattern: &str, backlinks_heading: &str) -> Result<NoteParser, &'static str> {
		let id_expr_str = format!(r"(?:\A|\s)({})(?:\z|\s)", &id_pattern);
		let id_expr = match Regex::new(&id_expr_str) {
			Ok(expr) => expr,
			Err(_) => return Err("Cannot parse ID format as regular expression"),
		};

		// Replace whitespace character representations
		let backlinks_heading = backlinks_heading
			.to_string()
			.replace("\\r", "\r")
			.replace("\\n", "\n")
			.replace("\\t", "\t");

		let wiki_link_format = "\\[\\[(?x)
			# Label can occur first, ends with |
			([^\\[\\]]+\\|)?
			(
				# Filename or ID
				{:link_chars:}+?
			)
			# Section can occur last, starts with #
			((?-x:#)[^\\[\\]]+)?
			\\]\\]"
			.replace("{:link_chars:}", "[^<>:*?/\\]\\[\"\\\\\\r\\n\\t]");

		/*
			Regular expressions below use "(?:\r|\n|\z)" instead of "$",
			since the latter for some reason doesn't match "\r"! This seems
			like a different implementation from other languages, or possibly
			some setting I'm unaware of.
		*/

		Ok(NoteParser {
			id_pattern: id_pattern.to_string(),
			id_expr,
			wiki_link_expr: Regex::new(&wiki_link_format).unwrap(),
			h1_expr: Regex::new(r"(?m)^#\s+(.+?)(?:\r|\n|\z)").unwrap(),
			todo_expr: Regex::new(r"(?m)^\s*[-*]\s+\[ \]\s+(.+?)(?:\r|\n|\z)").unwrap(),
			backlinks_heading: backlinks_heading,
			backlink_expr: Regex::new(r"(?m)^[-*]\s*(.*?)(?:\r|\n|\z)").unwrap(),
		})
	}

	fn get_id(&self, text: &str) -> Option<String> {
		match self.id_expr.captures(&text) {
			None => None,
			Some(capture) => Some(capture[1].to_string()),
		}
	}

	fn is_id(&self, text: &str) -> bool {
		self.id_expr.is_match(text)
	}

	fn remove_id(&self, text: &str) -> String {
		self.id_expr.replace(text, "").to_string()
	}

	fn get_h1(&self, text: &str) -> Option<String> {
		match self.h1_expr.captures(&text) {
			None => None,
			Some(capture) => Some(capture[1].to_string()),
		}
	}

	fn get_wiki_links(&self, text: &str) -> HashSet<WikiLink> {
		let mut links = HashSet::new();
		for capture in self.wiki_link_expr.captures_iter(&text) {
			let link = capture[2].to_string();
			if self.is_id(&link) {
				links.insert(WikiLink::Id(link));
			} else if !NoteFile::begins_or_ends_with_dot_or_space(&link) {
				links.insert(WikiLink::FileName(link));
			} else {
				dbg!(link);
			}
		}
		links
	}

	fn get_todos(&self, text: &str) -> Vec<String> {
		let mut todos = Vec::new();
		for capture in self.todo_expr.captures_iter(&text) {
			todos.push(capture[1].to_string());
		}
		todos
	}

	fn get_content_without_backlinks<'a>(&self, text: &'a str) -> &'a str {
		text.split(&self.backlinks_heading)
			.nth(0)
			.unwrap_or_default()
			.trim()
	}

	fn get_backlinks_section<'a>(&self, text: &'a str) -> &'a str {
		text.split(&self.backlinks_heading)
			.nth(1)
			.unwrap_or_default()
			.trim()
	}

	fn get_backlinks<'a>(&self, text: &'a str) -> Vec<String> {
		let mut backlinks = Vec::new();
		for capture in self
			.backlink_expr
			.captures_iter(self.get_backlinks_section(text))
		{
			backlinks.push(capture[1].to_string());
		}
		backlinks
	}
}

struct NoteCollection {
	/** Lookup for IDs and file names to all notes */
	notes: HashMap<WikiLink, Rc<Note>>,
	/** List of all notes */
	notes_iter: Vec<Rc<Note>>,
	backlinks: HashMap<WikiLink, Vec<Rc<Note>>>,
}

#[allow(dead_code)]
impl NoteCollection {
	fn collect_files(root: &str, extension: &str, parser: NoteParser) -> NoteCollection {
		let parser = Rc::new(parser);
		let mut notes = HashMap::new();
		let mut notes_iter = Vec::new();
		let mut backlinks = HashMap::new();

		let mut note_factory = |path: &path::PathBuf| {
			let path_ext = path.extension().unwrap_or_default().to_str().unwrap();
			if path_ext == extension {
				let note = Rc::new(Note::new(NoteFile::new(&path), Rc::clone(&parser)));
				if let Some(id) = &note.id {
					if let Some(conflicting_note) =
						notes.insert(WikiLink::Id(id.clone()), Rc::clone(&note))
					{
						eprintln!(
							"{} The id {} was used in both \"{}\" and \"{}\"",
							Colour::Yellow.paint("Warning:"),
							id,
							note.file.stem,
							conflicting_note.file.stem
						);
					}
				}
				notes.insert(
					WikiLink::FileName(note.file.stem.to_string()),
					Rc::clone(&note),
				);
				notes_iter.push(Rc::clone(&note));

				for link in &note.links {
					backlinks
						.entry(link.clone())
						.or_insert(Vec::new())
						.push(Rc::clone(&note));
				}
			}
		};

		visit_dirs(path::Path::new(root), &mut note_factory).expect("Error occurred!");

		NoteCollection {
			notes,
			notes_iter,
			backlinks,
		}
	}

	fn visit_notes(&self, callback: &mut dyn FnMut(&Note)) {
		// TODO: Add sorting callback?
		for note in &self.notes_iter {
			callback(&note);
		}
	}

	fn count(&self) -> usize {
		self.notes_iter.len()
	}

	fn count_with_id(&self) -> usize {
		let mut count: usize = 0;
		let mut f = |note: &Note| {
			if let Some(_) = &note.id {
				count += 1;
			}
		};
		self.visit_notes(&mut f);
		count
	}

	fn count_links(&self) -> usize {
		self.backlinks.len()
	}

	fn get_orphans(&self) -> Vec<NoteMeta> {
		let mut orphans = Vec::new();
		let mut f = |note: &Note| {
			if self
				.backlinks
				.contains_key(&WikiLink::FileName(note.file.stem.to_string()))
			{
				return;
			}

			if let Some(id) = &note.id {
				if self.backlinks.contains_key(&WikiLink::Id(id.to_string())) {
					return;
				}
			}

			orphans.push(note.get_meta());
		};
		self.visit_notes(&mut f);
		orphans
	}

	fn get_broken_links(&self) -> Vec<(&WikiLink, Vec<NoteMeta>)> {
		let mut notes = Vec::new();
		let linked: HashSet<&WikiLink> = self.backlinks.keys().collect();
		let existing: HashSet<&WikiLink> = self.notes.keys().collect();
		for broken in linked.difference(&existing) {
			let linkers: Vec<NoteMeta> = self.backlinks[broken]
				.iter()
				.map(|n| n.get_meta())
				.collect();
			notes.push((*broken, linkers));
		}
		notes
	}

	fn get_notes_without_links(&self) -> Vec<NoteMeta> {
		let mut notes = Vec::new();
		let mut f = |note: &Note| {
			if note.links.len() == 0 {
				notes.push(note.get_meta());
			}
		};
		self.visit_notes(&mut f);
		notes
	}

	fn get_todos(&self) -> HashMap<NoteMeta, Vec<String>> {
		let mut todos = HashMap::new();
		let mut f = |note: &Note| {
			if note.todos.len() > 0 {
				todos.insert(note.get_meta(), note.todos.clone());
			}
		};
		self.visit_notes(&mut f);
		todos
	}

	fn update_backlinks(&self) {}

	fn get_mismatched_filenames(&self) -> Vec<(NoteMeta, String)> {
		let mut fs = Vec::new();
		let mut f = |note: &Note| {
			let new_filename = if let Some(id) = &note.id {
				NoteFile::escape_filename(&format!("{} {}", id, &note.title))
			} else {
				NoteFile::escape_filename(&note.title)
			};

			if note.file.stem != new_filename {
				fs.push((
					note.get_meta(),
					format!("{}.{}", new_filename, &note.file.extension),
				));
			}
		};
		self.visit_notes(&mut f);
		fs
	}
}

fn visit_dirs(path: &path::Path, callback: &mut dyn FnMut(&path::PathBuf)) -> io::Result<()> {
	dbg!(path);
	if path.is_dir() {
		for entry in fs::read_dir(path)? {
			let entry = entry?;
			let path = entry.path();
			let first_letter = path
				.file_name()
				.unwrap()
				.to_str()
				.unwrap()
				.chars()
				.nth(0)
				.unwrap();

			if first_letter == '.' {
				// Ignore "hidden" files
				continue;
			}

			if path.is_dir() {
				visit_dirs(&path, callback)?;
			} else if path.is_file() {
				callback(&path);
			}
		}
	}
	Ok(())
}

#[derive(Debug)]
pub struct Config {
	pub id_pattern: String,
	pub backlinks_heading: String,
	pub extension: String,
	pub path: String,
	pub command: String,
}

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
	let notes = NoteCollection::collect_files(
		&config.path,
		&config.extension,
		NoteParser::new(&config.id_pattern, &config.backlinks_heading)?,
	);

	match config.command.as_str() {
		"list-broken-links" => print_broken_links(&notes),
		"list-orphans" => print_orphans(&notes),
		"list-todos" => print_todos(&notes),
		"update-backlinks" => update_backlinks(&notes)?,
		"update-filenames" => update_filenames(&notes)?,
		_ => print_stats(&notes),
	}

	Ok(())
}

fn print_stats(notes: &NoteCollection) {
	println!("# {}\n", Style::new().bold().paint("Statistics"));

	println!("- Found number of notes: {}", notes.count());
	println!("- Found number of note IDs: {}", notes.count_with_id());
	println!("- Found number of links: {}", notes.count_links());

	// TODO: Add number of written words and characters (without whitespace)
}

fn print_todos(notes: &NoteCollection) {
	println!("# {}\n", Style::new().bold().paint("To-do"));

	for (note, todos) in notes.get_todos() {
		println!("\n## {}\n", note.get_wikilink_to());

		for todo in todos {
			println!("- [ ] {}", todo);
		}
	}
}

fn print_orphans(notes: &NoteCollection) {
	println!("# {}\n", Style::new().bold().paint("Orphans"));

	for note in notes.get_orphans() {
		println!("- {}", note.get_wikilink_to());
	}
}

fn print_broken_links(notes: &NoteCollection) {
	println!("# {}\n", Style::new().bold().paint("Broken links"));

	for (link, notes) in notes.get_broken_links() {
		let linkers: Vec<String> = notes.iter().map(|n| n.get_wikilink_to()).collect();
		println!("- \"{}\" links to unknown {}", linkers.join(" and "), link);
	}
}

fn update_backlinks(notes: &NoteCollection) -> io::Result<()> {
	Ok(())
}

fn update_filenames(notes: &NoteCollection) -> io::Result<()> {
	for (note, new_filename) in notes.get_mismatched_filenames() {
		let original_file_name = format!("{}.{}", note.stem, note.extension);
		let reply = rprompt::prompt_reply_stdout(&format!(
			"Rename \"{}\" to \"{}\"? ([y]/n) ",
			original_file_name, new_filename
		))?;

		if reply == "y" || reply == "" {
			if note.path.ends_with(&original_file_name) {
				let folder = &note.path[..(note.path.len() - original_file_name.len())];
				let new_path = folder.to_string() + &new_filename;
				fs::rename(note.path, new_path)?;
			} else {
				// TODO: Return as Err
				eprintln!("Error: probably a bug in how the file name path is determined");
			}
		}
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use crate::*;

	fn get_default_parser() -> NoteParser {
		NoteParser::new(
			r"\d{11,14}",
			"-----------------\\r\\n**Links to this note**",
		)
		.expect("Test parser failed")
	}

	#[test]
	fn title_and_id_parser() {
		let parser = Rc::new(get_default_parser());
		let note = Note::new(
			NoteFile::new(&path::PathBuf::from(
				r"testdata/12345678901 File Name Title.md",
			)),
			Rc::clone(&parser),
		);

		assert_eq!(note.file.stem, "12345678901 File Name Title");
		assert_eq!(note.id.unwrap(), "1234567890123");
		assert_eq!(note.title, "The Title In the Note Contents");
	}

	#[test]
	fn empty_file_parser() {
		let parser = Rc::new(get_default_parser());
		let note = Note::new(
			NoteFile::new(&path::PathBuf::from(r"testdata/Empty File With Name.md")),
			Rc::clone(&parser),
		);

		assert_eq!(note.file.stem, "Empty File With Name");
		assert_eq!(note.id, None);
		assert_eq!(note.title, "Empty File With Name");
	}

	#[test]
	fn title_parser() {
		let parser = get_default_parser();
		let note_file = NoteFile::new(&path::PathBuf::from(r"testdata/12345678901 Test Note 1.md"));
		let title = Note::get_title_assoc(&note_file, &parser);
		assert_eq!(title, "Test Note 1");
	}

	#[test]
	fn oneliner_parser() {
		let parser = get_default_parser();
		let note_file = NoteFile::new(&path::PathBuf::from(r"testdata/One-liner.md"));
		let title = Note::get_title_assoc(&note_file, &parser);
		assert_eq!(title, "Just a Heading");
	}

	#[test]
	fn link_parser() {
		let parser = Rc::new(get_default_parser());
		let note = Note::new(
			NoteFile::new(&path::PathBuf::from(r"testdata/Links.md")),
			Rc::clone(&parser),
		);

		// dbg!(&note.links);

		let expected_links = vec![
			WikiLink::Id("20210104073402".to_string()),
			WikiLink::Id("20210103212011".to_string()),
			WikiLink::FileName("Filename Link".to_string()),
			WikiLink::FileName("Search Query Link".to_string()),
			WikiLink::FileName("Regular Link To Wiki URI".to_string()),
			WikiLink::FileName("Inside Fenced Code Block".to_string()),
			WikiLink::FileName("labelling wiki links".to_string()),
			WikiLink::FileName("the filename first".to_string()),
			WikiLink::FileName("a note".to_string()),
			WikiLink::FileName("Stars and stripes".to_string()),
			WikiLink::FileName("Stars or stripes".to_string()),
			WikiLink::FileName("link 123".to_string()),
			WikiLink::FileName("link 234".to_string()),
		];

		for expected_link in &expected_links {
			assert!(note.links.contains(expected_link));
		}
		assert_eq!(note.links.len(), expected_links.len());
	}

	#[test]
	fn todo_parser() {
		let parser = Rc::new(get_default_parser());
		let note = Note::new(
			NoteFile::new(&path::PathBuf::from(r"testdata/Todos.md")),
			Rc::clone(&parser),
		);

		assert!(note.todos.contains(&"Don't forget to remember".to_string()));
		assert!(note.todos.contains(&"Buy milk!".to_string()));
		assert!(note.todos.contains(&"Nested".to_string()));
		assert!(note.todos.contains(&"Tabbed".to_string()));
		assert!(note.todos.contains(&"Final line".to_string()));
		assert_eq!(note.todos.len(), 5);
	}

	#[test]
	fn wikilink_traits() {
		assert_ne!(
			WikiLink::Id("1234567890".to_string()),
			WikiLink::Id("0987654321".to_string())
		);
		assert_ne!(
			WikiLink::FileName("The name".to_string()),
			WikiLink::Id("Le nom".to_string())
		);
		assert_ne!(
			WikiLink::FileName("1234567890".to_string()),
			WikiLink::Id("1234567890".to_string())
		);
		assert_eq!(
			WikiLink::Id("1234567890".to_string()),
			WikiLink::Id("1234567890".to_string())
		);
		assert_eq!(
			WikiLink::FileName("AÉÖÜÅÑ".to_string()),
			WikiLink::FileName("aéöüåñ".to_string())
		);

		let mut map = HashMap::new();
		map.insert(WikiLink::Id("1234567890".to_string()), "Some value");
		map.insert(WikiLink::Id("9876543210".to_string()), "Some value");
		map.insert(WikiLink::FileName("1234567890".to_string()), "Some value");
		map.insert(WikiLink::FileName("ÅSTRÖM".to_string()), "Some value");
		map.insert(WikiLink::FileName("åström".to_string()), "Some value");
		map.insert(WikiLink::FileName("Astrom".to_string()), "Some value");
		assert_eq!(map.len(), 5);
	}

	#[test]
	fn backlinks() {
		let parser = Rc::new(get_default_parser());
		let note = Note::new(
			NoteFile::new(&path::PathBuf::from(r"testdata/Backlinks.md")),
			Rc::clone(&parser),
		);

		// All links in this file is in the backlinks section
		assert_eq!(note.links.len(), 0);

		let backlinks = parser.get_backlinks(&note.file.content);
		assert!(backlinks.contains(&"[[§An outline note]]".to_string()));
		assert!(backlinks.contains(&"[[20201012145848]] Another note".to_string()));
		assert!(backlinks.contains(&"Not a link".to_string()));
		assert_eq!(backlinks.len(), 3);
	}

	#[test]
	fn escape_filename() {
		assert_eq!(
			NoteFile::escape_filename("Just a normal file name"),
			"Just a normal file name"
		);
		assert_eq!(
			NoteFile::escape_filename("<Is/this\\a::regular?*?*?file>"),
			"Is this a regular file"
		);
		assert_eq!(NoteFile::escape_filename(".hidden file"), "hidden file");
		assert_eq!(
			NoteFile::escape_filename("illegal in windows."),
			"illegal in windows"
		);
		assert_eq!(
			NoteFile::escape_filename("a . in the middle"),
			"a . in the middle"
		);
		assert_eq!(NoteFile::escape_filename(".:/?."), "");
	}
}

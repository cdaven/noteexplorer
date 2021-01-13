use std::marker::PhantomData;
use std::error::Error;
use regex::Regex;
use std::collections::HashMap;
use std::collections::HashSet;
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
}

#[derive(Debug)]
struct Note<'a> {
	file: NoteFile,
	title: Option<String>,
	id: Option<String>,
	links: HashSet<WikiLink>,
	todos: Vec<String>,
	parser: Rc<NoteParser>,
	phantom: &'a str
}

// Use path as unique identifier for notes
impl<'a> PartialEq for Note<'a> {
	fn eq(&self, other: &Self) -> bool {
		self.file.path == other.file.path
	}
}

impl<'a> Eq for Note<'a> {}

// Use path as unique identifier for notes
impl<'a> Hash for Note<'a> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.file.path.hash(state);
	}
}

impl<'a> Note<'a> {
	fn new(file: NoteFile, parser: Rc<NoteParser>) -> Note<'a> {
		Note {
			id: Note::get_id_assoc(&file, &parser),
			title: Note::get_title_assoc(&file, &parser),
			links: Note::get_links_assoc(&file, &parser),
			todos: Note::get_todos_assoc(&file, &parser),
			parser,
			file,
			phantom: ""
		}
	}

	fn get_id(&self) -> &Option<String> {
		&self.id
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

	fn get_file_stem(&self) -> &str {
		&self.file.stem
	}

	fn get_title(&self) -> &Option<String> {
		&self.title
	}

	fn get_title_assoc(file: &NoteFile, parser: &NoteParser) -> Option<String> {
		// Try first with a H1 title in the contents
		if let Some(title) = parser.get_h1(&file.content) {
			Some(title)
		} else {
			// Then use file stem without ID (even if the wrong ID)
			Some(parser.remove_id(&file.stem).trim().to_string())
		}
	}

	fn get_links(&self) -> &HashSet<WikiLink> {
		&self.links
	}

	fn get_links_assoc(file: &NoteFile, parser: &NoteParser) -> HashSet<WikiLink> {

		parser.get_wiki_links(&file.content)
	}

	fn get_todos(&self) -> &Vec<String> {
		&self.todos
	}

	fn get_todos_assoc(file: &NoteFile, parser: &NoteParser) -> Vec<String> {
		parser.get_todos(&file.content)
	}

	fn get_content_without_backlinks(file: &'a NoteFile, parser: &NoteParser) -> &'a str {
		file.content.split(&parser.backlinks_heading).nth(0).unwrap_or_default()
	}

	fn get_backlinks_section(file: &'a NoteFile, parser: &NoteParser) -> &'a str {
		file.content.split(&parser.backlinks_heading).nth(1).unwrap_or_default()
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
pub struct NoteMeta {
	path: String,
	stem: String,
	extension: String,
	title: Option<String>,
	id: Option<String>,
}

impl NoteMeta {
	pub fn get_wikilink_to(&self) -> String {
		let empty_str = String::from("");
		let id = self.id.as_ref().unwrap_or(&self.stem);
		let mut title = self.title.as_ref().unwrap_or(&empty_str);

		// When the id and title are the same, don't repeat the title
		if title == id {
			title = &empty_str;
		}

		format!("[[{}]] {}", id, title).trim_end().to_string()
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

#[derive(Debug)]
pub struct NoteParser {
	id_pattern: String,
	id_expr: Regex,
	wiki_link_expr: Regex,
	h1_expr: Regex,
	todo_expr: Regex,
	backlinks_heading: String,
}

impl NoteParser {
	pub fn new(id_pattern: &str, backlinks_heading: &str) -> NoteParser {
		let id_expr_str = format!(r"(?:\A|\s)({})(?:\z|\s)", &id_pattern);

		/*
			Regular expressions below use "(?:\r|\n|\z)" instead of "$",
			since the latter for some reason doesn't match "\r"! This seems
			like a different implementation from other languages, or possibly
			some setting I'm unaware of.
		*/

		NoteParser {
			id_pattern: id_pattern.to_string(),
			id_expr: Regex::new(&id_expr_str).unwrap(),
			wiki_link_expr: Regex::new(r"\[\[([^\]\[]+?)\]\]").unwrap(),
			h1_expr: Regex::new(r"(?m)^#\s+(.+?)(?:\r|\n|\z)").unwrap(),
			todo_expr: Regex::new(r"(?m)^\s*[-*] \[ \]\s*(.+?)(?:\r|\n|\z)").unwrap(),
			backlinks_heading: backlinks_heading.to_string(),
		}
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
			let link = capture[1].to_string();
			if self.is_id(&link) {
				links.insert(WikiLink::Id(link));
			} else {
				links.insert(WikiLink::FileName(link));
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
}

pub struct NoteCollection<'a> {
	/** Lookup for IDs and file names to all notes */
	notes: HashMap<WikiLink, Rc<Note<'a>>>,
	/** List of all notes */
	notes_iter: Vec<Rc<Note<'a>>>,
	backlinks: HashMap<WikiLink, Vec<Rc<Note<'a>>>>,
	phantom: &'a str
}

impl<'a> NoteCollection<'a> {
	pub fn collect_files(root: &str, extension: &str, parser: NoteParser) -> NoteCollection<'a> {
		println!(
			"Collecting notes from {} with extension {}",
			root, extension
		);
		let parser = Rc::new(parser);
		let mut notes = HashMap::new();
		let mut notes_iter = Vec::new();
		let mut backlinks = HashMap::new();

		let mut note_factory = |path: &path::PathBuf| {
			let path_ext = path.extension().unwrap_or_default().to_str().unwrap();
			if path_ext == extension {
				let note = Rc::new(Note::new(NoteFile::new(&path), Rc::clone(&parser)));
				println!(
					"Parsing note {:?} with id {:?}",
					note.get_file_stem(),
					note.id
				);

				if let Some(id) = note.get_id() {
					notes.insert(WikiLink::Id(id.clone()), Rc::clone(&note));
				}
				notes.insert(
					WikiLink::FileName(note.get_file_stem().to_string()),
					Rc::clone(&note),
				);
				notes_iter.push(Rc::clone(&note));

				for link in note.get_links() {
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
			phantom: ""
		}
	}

	pub fn count(&self) -> usize {
		self.notes_iter.len()
	}

	fn visit_notes(&self, callback: &mut dyn FnMut(&Note)) {
		// TODO: Add sorting callback?
		for note in &self.notes_iter {
			callback(&note);
		}
	}

	pub fn get_orphans(&self) -> Vec<NoteMeta> {
		let mut orphans = Vec::new();
		let mut f = |note: &Note| {
			if self
				.backlinks
				.contains_key(&WikiLink::FileName(note.get_file_stem().to_string()))
			{
				return;
			}

			if let Some(id) = note.get_id() {
				if self.backlinks.contains_key(&WikiLink::Id(id.to_string())) {
					return;
				}
			}

			orphans.push(note.get_meta());
		};
		self.visit_notes(&mut f);
		orphans
	}

	pub fn get_broken_links(&self) {}

	pub fn get_notes_without_links(&self) -> Vec<NoteMeta> {
		let mut notes = Vec::new();
		let mut f = |note: &Note| {
			if note.get_links().len() == 0 {
				notes.push(note.get_meta());
			}
		};
		self.visit_notes(&mut f);
		notes
	}

	pub fn get_todos(&self) -> HashMap<NoteMeta, Vec<String>> {
		let mut todos = HashMap::new();
		let mut f = |note: &Note| {
			if note.get_todos().len() > 0 {
				todos.insert(note.get_meta(), note.get_todos().clone());
			}
		};
		self.visit_notes(&mut f);
		todos
	}

	pub fn update_backlinks(&self) {}

	pub fn update_filenames(&self) {}
}

fn visit_dirs(path: &path::Path, callback: &mut dyn FnMut(&path::PathBuf)) -> io::Result<()> {
	println!("Visiting directory {:?}", path.as_os_str());
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
	pub path: String
}

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
	let parser = NoteParser::new(&config.id_pattern, &config.backlinks_heading);
	let notes = NoteCollection::collect_files(&config.path, &config.extension, parser);

	println!("Collected {} notes", notes.count());
	print_todos(&notes);
	print_orphans(&notes);

	Ok(())
}

fn print_todos(notes: &NoteCollection) {
	println!("# TODOs");

	for (note, todos) in notes.get_todos() {
		println!("\n## {}\n", note.get_wikilink_to());

		for todo in todos {
			println!("- [ ] {}", todo);
		}
	}
}

fn print_orphans(notes: &NoteCollection) {
	println!("# Orphans\n");

	for note in notes.get_orphans() {
		println!("- {}", note.get_wikilink_to());
	}
}


#[cfg(test)]
mod tests {
	use crate::*;

	fn get_default_parser() -> NoteParser {
		NoteParser::new(r"\d{11,14}", r"-----------------\r\n**Links to this note**")
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

		assert_eq!(note.get_file_stem(), "12345678901 File Name Title");
		assert_eq!(*note.get_id().as_ref().unwrap(), "1234567890123");
		assert_eq!(
			*note.get_title().as_ref().unwrap(),
			"The Title In the Note Contents"
		);
	}

	#[test]
	fn empty_file_parser() {
		let parser = Rc::new(get_default_parser());
		let note = Note::new(
			NoteFile::new(&path::PathBuf::from(r"testdata/Empty File With Name.md")),
			Rc::clone(&parser),
		);

		assert_eq!(note.get_file_stem(), "Empty File With Name");
		assert_eq!(note.get_id().as_ref(), None);
		assert_eq!(*note.get_title().as_ref().unwrap(), "Empty File With Name");
	}

	#[test]
	fn title_parser() {
		let parser = get_default_parser();
		let note_file = NoteFile::new(&path::PathBuf::from(r"testdata/12345678901 Test Note 1.md"));
		let title = Note::get_title_assoc(&note_file, &parser);
		assert_eq!(title.as_ref().unwrap(), "Test Note 1");
	}

	#[test]
	fn oneliner_parser() {
		let parser = get_default_parser();
		let note_file = NoteFile::new(&path::PathBuf::from(r"testdata/One-liner.md"));
		let title = Note::get_title_assoc(&note_file, &parser);
		assert_eq!(title.as_ref().unwrap(), "Just a Heading");
	}

	#[test]
	fn link_parser() {
		let parser = Rc::new(get_default_parser());
		let note = Note::new(
			NoteFile::new(&path::PathBuf::from(r"testdata/Links.md")),
			Rc::clone(&parser),
		);

		assert!(note
			.links
			.contains(&WikiLink::Id("20210104073402".to_string())));
		assert!(note
			.links
			.contains(&WikiLink::Id("20210103212011".to_string())));
		assert!(note
			.links
			.contains(&WikiLink::FileName("Filename Link".to_string())));
		assert!(note
			.links
			.contains(&WikiLink::FileName("Search Query Link".to_string())));
		assert!(note
			.links
			.contains(&WikiLink::FileName("Regular Link To Wiki URI".to_string())));
		assert!(note
			.links
			.contains(&WikiLink::FileName("#my-custom-id".to_string())));
		assert!(note
			.links
			.contains(&WikiLink::FileName("Inside Fenced Code Block".to_string())));
		assert_eq!(note.links.len(), 7);
	}

	#[test]
	fn todo_parser() {
		let parser = Rc::new(get_default_parser());
		let note = Note::new(
			NoteFile::new(&path::PathBuf::from(r"testdata/Todos.md")),
			Rc::clone(&parser),
		);

		println!("{:?}", note.todos);

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
	fn string_find() {
		let s = "Hello, world!";
		let parts: Vec<&str> = s.split(", ").collect();

		assert_eq!(parts[0], "Hello");
		assert_eq!(parts[1], "world!");
	}
}

use crate::ftree;
use crate::mdparse::NoteParser;
use ansi_term::Colour;
use chrono::Utc;
use debug_print::debug_println;
use lazy_static::*;
use regex::Regex;
use std::cell::{Ref, RefCell};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::iter::FromIterator;
use std::rc::Rc;
use std::{fs, io, path};

lazy_static! {
	static ref EMPTY_STRING: String = String::from("");
	// These characters are replaced with " " (illegal in Windows)
	static ref ILLEGAL_FILE_CHARS: Regex = Regex::new("[<>:*?|/\"\\\\\\t\\r\\n]").unwrap();
	// "." at the beginning or end are removed
	static ref SURROUNDING_DOTS: Regex = Regex::new(r"(\A\.|\.\z)").unwrap();
	// Replace double spaces with single
	static ref DOUBLE_SPACES: Regex = Regex::new(r" +").unwrap();
}

#[derive(Debug)]
pub struct NoteFile {
	/// Full path to file
	pub path: String,
	/// Filename without path and extension
	pub stem: String,
	/// Filename extension without leading dot
	pub extension: String,
	/// File contents
	pub content: String,
}

impl NoteFile {
	fn new(path: &path::PathBuf) -> Result<NoteFile, io::Error> {
		Ok(NoteFile {
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
			content: fs::read_to_string(&path)?,
		})
	}

	/** Clean filename to comply with Windows, OSX and Linux rules, plus the extra rule that filenames don't start with dots or have leading spaces */
	fn clean_filename(filename: &str) -> String {
		DOUBLE_SPACES
			.replace_all(
				&SURROUNDING_DOTS
					.replace_all(
						&ILLEGAL_FILE_CHARS.replace_all(&filename, " ").to_string(),
						"",
					)
					.to_string(),
				" ",
			)
			.trim()
			.to_string()
	}

	pub fn save(path: &str, contents: &str) -> io::Result<()> {
		// Make sure file always ends with one newline
		fs::write(&path, String::from(contents.trim_end()) + "\n")
	}

	/// Renames file, assuming that the path is valid and escaped
	pub fn rename(&self, new_stem: &str) -> io::Result<NoteFile> {
		let new_path = path::Path::new(&self.path)
			.with_file_name(new_stem)
			.with_extension(&self.extension);
		fs::rename(&self.path, &new_path)?;
		Ok(NoteFile {
			path: new_path.as_os_str().to_str().unwrap().to_string(),
			stem: new_stem.to_string(),
			extension: self.extension.clone(),
			content: self.content.clone(),
		})
	}

	pub fn replace_contents(&self, contents: &str) -> NoteFile {
		NoteFile {
			path: self.path.clone(),
			stem: self.stem.clone(),
			extension: self.extension.clone(),
			content: contents.to_owned(),
		}
	}
}

#[derive(Debug)]
struct Note {
	file: NoteFile,
	title: String,
	title_lower: String,
	id: Option<String>,
	links: HashSet<WikiLink>,
	tasks: Vec<String>,
	backlinks_start: Option<usize>,
	backlinks_end: Option<usize>,
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

type RcRefNote = Rc<RefCell<Note>>;

impl Note {
	fn new(file: NoteFile, parser: Rc<NoteParser>) -> Note {
		let data = parser.parse(&file.content);

		let id = match parser.get_id(&file.stem) {
			// Prefer ID from filename if it exists
			Some(file_id) => Some(file_id),
			None => data.ids.into_iter().next(),
		};

		let title = if !data.titles.is_empty() {
			// Prefer title from contents
			data.titles.into_iter().next().unwrap_or_default()
		} else {
			// Fall back to filename minus ID
			parser.remove_id(&file.stem)
		};

		Note {
			id,
			title_lower: title.to_lowercase(),
			title,
			links: HashSet::from_iter(data.links),
			tasks: data.tasks,
			backlinks_start: data.backlinks_start,
			backlinks_end: data.backlinks_end,
			parser,
			file,
		}
	}

	/// Insert/replace NoteFile object in mutable copy
	fn insert_file(&mut self, file: NoteFile) {
		self.file = file
	}

	fn has_backlinks(&self) -> bool {
		self.backlinks_start.is_some()
	}

	/// Returns note contents with the backlinks section left out.
	fn get_contents_without_backlinks(&self) -> String {
		if let Some(start) = self.backlinks_start {
			let end = self
				.backlinks_end
				.unwrap_or_else(|| self.file.content.len());

			let new_len = start + self.file.content.len() - end;
			let mut contents = String::with_capacity(new_len);
			contents.push_str(&self.file.content[..start]);
			if end < self.file.content.len() {
				contents.push_str(&self.file.content[end..]);
			}
			assert_eq!(contents.len(), new_len);
			contents
		} else {
			self.file.content.to_owned()
		}
	}

	/// Returns note contents with the backlinks section switched or added
	fn get_contents_with_new_backlinks(&self, heading: &str, backlinks: &str) -> String {
		let make_contents = |before: &str, after: &str| {
			[before.trim_end(), heading, backlinks, after]
				.join("\n\n")
				.trim_end()
				.to_owned()
		};

		if let Some(start) = self.backlinks_start {
			let end = self
				.backlinks_end
				.unwrap_or_else(|| self.file.content.len());

			make_contents(&self.file.content[..start], &self.file.content[end..])
		} else {
			make_contents(&self.file.content, &"")
		}
	}

	/// Returns backlinks section without the heading, trimmed
	fn get_backlinks_section_without_heading(&self) -> Option<&str> {
		if let Some(start) = self.backlinks_start {
			let end = self
				.backlinks_end
				.unwrap_or_else(|| self.file.content.len());

			Some(&self.file.content[start + self.parser.backlinks_heading.len()..end].trim())
		} else {
			None
		}
	}

	fn has_outgoing_links(&self) -> bool {
		!self.links.is_empty()
	}

	/// Return a copy of the note's meta data
	fn get_meta(&self) -> NoteMeta {
		NoteMeta {
			path: self.file.path.clone(),
			stem: self.file.stem.clone(),
			extension: self.file.extension.clone(),
			title: self.title.clone(),
			id: self.id.clone(),
		}
	}

	fn get_wikilink_to(&self) -> String {
		Note::get_wikilink(&self.id, &self.title, &self.file.stem)
	}

	fn get_wikilink(id: &Option<String>, title: &str, file_stem: &str) -> String {
		// Link either to ID or filename
		let link_target = if let Some(i) = id { i } else { file_stem };

		let link_desc = if title == link_target {
			// No need for a link description that matches the link target
			// (E.g. "[[Filename link]] Filename link")
			&EMPTY_STRING
		} else {
			title
		};

		format!("[[{}]] {}", link_target, link_desc)
			.trim_end()
			.to_string()
	}

	pub fn get_filename_link(&self) -> WikiLink {
		WikiLink::FileName(self.file.stem.to_string())
	}

	fn is_link_to(&self, link: &WikiLink) -> bool {
		match link {
			WikiLink::FileName(filename) => {
				self.file.stem.to_lowercase() == filename.to_lowercase()
			}
			WikiLink::Id(id) => {
				self.id.as_ref().unwrap_or(&EMPTY_STRING).to_lowercase() == id.to_lowercase()
			}
		}
	}

	pub fn save(&self) -> io::Result<()> {
		NoteFile::save(&self.file.path, &self.file.content)
	}
}

#[derive(PartialEq, Eq, Hash, Debug)]
pub struct NoteMeta {
	pub path: String,
	pub stem: String,
	pub extension: String,
	pub title: String,
	pub id: Option<String>,
}

impl NoteMeta {
	pub fn get_wikilink_to(&self) -> String {
		Note::get_wikilink(&self.id, &self.title, &self.stem)
	}
}

#[derive(Eq, Clone, Debug)]
pub enum WikiLink {
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

pub struct NoteCollection {
	/// Lookup for IDs and file names to all notes
	notes: HashMap<WikiLink, RcRefNote>,
	/// Lookup for links, with the target as key
	backlinks: HashMap<WikiLink, Vec<RcRefNote>>,
}

impl NoteCollection {
	pub fn collect_files(root: &path::Path, extension: &str, parser: NoteParser) -> NoteCollection {
		let parser = Rc::new(parser);
		let mut notes = HashMap::new();
		let mut backlinks = HashMap::new();

		let start_time = Utc::now();
		let note_paths = ftree::get_files(root, extension);
		let duration_get_files = Utc::now() - start_time;
		let start_time = Utc::now();
		for path in note_paths {
			let note_file = match NoteFile::new(&path) {
				Ok(nf) => nf,
				Err(err) => {
					eprintln!(
						"{} Couldn't read file {}: {}",
						Colour::Red.paint("Error:"),
						path.to_string_lossy(),
						err
					);
					continue;
				}
			};

			let note = Rc::new(RefCell::new(Note::new(note_file, Rc::clone(&parser))));

			if let Some(id) = &note.borrow().id {
				if let Some(conflicting_note) =
					notes.insert(WikiLink::Id(id.clone()), Rc::clone(&note))
				{
					eprintln!(
						"{} The id {} was used in both \"{}\" and \"{}\"",
						Colour::Yellow.paint("Warning:"),
						id,
						note.borrow().file.stem,
						conflicting_note.borrow().file.stem
					);
				}
			}

			notes.insert(note.borrow().get_filename_link(), Rc::clone(&note));

			for link in &note.borrow().links {
				// Ignore "backlinks" to self
				if !note.borrow().is_link_to(link) {
					backlinks
						.entry(link.clone())
						.or_insert_with(Vec::new)
						.push(Rc::clone(&note));
				}
			}
		}
		let duration_note_loop = Utc::now() - start_time;

		debug_println!(
			"ftree::get_files() took {} ms",
			duration_get_files.num_milliseconds()
		);
		debug_println!(
			"loading and parsing notes took {} ms",
			duration_note_loop.num_milliseconds()
		);

		NoteCollection { notes, backlinks }
	}

	/// Get iterator over notes
	fn get_notes_iter(&self) -> impl Iterator<Item = Ref<Note>> {
		self.notes
			.iter()
			// Only look at filename ids/keys, since all notes have
			// exactly one filename, but 0..1 ids.
			.filter(|(k, _)| matches!(k, WikiLink::FileName(_)))
			.map(|(_, v)| v.borrow())
	}

	/// Get vector of notes, sorted by title
	fn get_sorted_notes(&self) -> Vec<Ref<Note>> {
		let mut notes: Vec<Ref<Note>> = self.get_notes_iter().collect();
		notes.sort_by(|a, b| a.title_lower.cmp(&b.title_lower));
		notes
	}

	pub fn count(&self) -> usize {
		self.get_notes_iter().count()
	}

	pub fn count_with_id(&self) -> usize {
		self.get_notes_iter().filter(|n| n.id.is_some()).count()
	}

	pub fn count_links(&self) -> usize {
		self.backlinks.len()
	}

	pub fn into_meta_vec(&self) -> Vec<NoteMeta> {
		let mut notes = Vec::with_capacity(self.count());
		for note in &self.get_sorted_notes() {
			notes.push(note.get_meta());
		}
		notes
	}

	fn note_has_incoming_links(&self, note: &Note) -> bool {
		if self
			.backlinks
			.contains_key(&WikiLink::FileName(note.file.stem.to_string()))
		{
			return true;
		}

		if let Some(id) = &note.id {
			if self.backlinks.contains_key(&WikiLink::Id(id.to_string())) {
				return true;
			}
		}

		false
	}

	/// Get incoming links to note. Can contain duplicates!
	fn get_incoming_links(&self, note: &Note) -> Vec<RcRefNote> {
		let empty: Vec<RcRefNote> = Vec::new();

		let mut links: Vec<&RcRefNote> = self
			.backlinks
			.get(&note.get_filename_link())
			.unwrap_or(&empty)
			.iter()
			.collect();

		if let Some(id) = &note.id {
			links.extend(
				&mut self
					.backlinks
					.get(&WikiLink::Id(id.to_string()))
					.unwrap_or(&empty)
					.iter(),
			)
		}

		links.iter().map(|rcn| Rc::clone(rcn)).collect()
	}

	/// Get notes with no incoming links, but at least one outgoing
	pub fn get_sources(&self) -> Vec<NoteMeta> {
		let mut sources = Vec::new();
		for note in &self.get_sorted_notes() {
			if note.has_outgoing_links() && !self.note_has_incoming_links(note) {
				sources.push(note.get_meta());
			}
		}
		sources
	}

	/** Get notes with no outgoing links, but at least one incoming */
	pub fn get_sinks(&self) -> Vec<NoteMeta> {
		let mut sinks = Vec::new();
		for note in &self.get_sorted_notes() {
			if !note.has_outgoing_links() && self.note_has_incoming_links(note) {
				sinks.push(note.get_meta());
			}
		}
		sinks
	}

	/** Get notes with no incoming or outgoing links */
	pub fn get_isolated(&self) -> Vec<NoteMeta> {
		let mut isolated = Vec::new();
		for note in &self.get_sorted_notes() {
			if !note.has_outgoing_links() && !self.note_has_incoming_links(note) {
				isolated.push(note.get_meta());
			}
		}
		isolated
	}

	pub fn get_broken_links(&self) -> Vec<(&WikiLink, Vec<NoteMeta>)> {
		let mut notes = Vec::new();
		let linked: HashSet<&WikiLink> = self.backlinks.keys().collect();
		let existing: HashSet<&WikiLink> = self.notes.keys().collect();
		for broken in linked.difference(&existing) {
			let linkers: Vec<NoteMeta> = self.backlinks[broken]
				.iter()
				.map(|note| note.borrow().get_meta())
				.collect();
			notes.push((*broken, linkers));
		}
		notes
	}

	pub fn get_tasks(&self) -> Vec<(NoteMeta, Vec<String>)> {
		let mut tasks = Vec::new();
		for note in &self.get_sorted_notes() {
			if !note.tasks.is_empty() {
				tasks.push((note.get_meta(), note.tasks.clone()));
			}
		}
		tasks
	}

	pub fn remove_backlinks(&self) -> Vec<NoteMeta> {
		let mut notes = Vec::new();
		for note in &self.get_sorted_notes() {
			if note.has_backlinks() {
				if let Err(e) =
					NoteFile::save(&note.file.path, &note.get_contents_without_backlinks())
				{
					eprintln!("Error while saving note file {}: {}", note.file.path, e);
				} else {
					notes.push(note.get_meta());
				}
			}
		}
		notes
	}

	pub fn update_backlinks(&self) -> Vec<NoteMeta> {
		let mut notes = Vec::new();
		for note in &self.get_sorted_notes() {
			let incoming_links = self.get_incoming_links(note);
			let mut incoming_links: Vec<Ref<Note>> =
				incoming_links.iter().map(|n| n.borrow()).collect();

			// First sort by filename to get a stable sort when titles are identical
			incoming_links.sort_by(|a, b| a.file.stem.cmp(&b.file.stem));
			incoming_links.sort_by(|a, b| a.title_lower.cmp(&b.title_lower));

			let mut new_backlinks: Vec<String> = incoming_links
				.iter()
				.map(|linking_note| "- ".to_string() + &linking_note.get_wikilink_to())
				.collect();

			// Remove possible duplicate links
			new_backlinks.dedup();

			let new_section = new_backlinks.join("\n");

			let current_section = note
				.get_backlinks_section_without_heading()
				.unwrap_or_default();

			if current_section != new_section {
				let new_contents = if !new_section.is_empty() {
					// Add or update backlinks
					note.get_contents_with_new_backlinks(
						&note.parser.backlinks_heading,
						&new_section,
					)
				} else {
					// Remove backlinks
					note.get_contents_without_backlinks()
				};
				if let Err(e) = NoteFile::save(&note.file.path, &new_contents) {
					eprintln!("Error while saving note file {}: {}", note.file.path, e);
				} else {
					notes.push(note.get_meta());
				}
			}
		}
		notes
	}

	pub fn get_mismatched_filenames(&self) -> Vec<(NoteMeta, String)> {
		let mut fs = Vec::new();
		for note in &self.get_sorted_notes() {
			let new_filename = if let Some(id) = &note.id {
				NoteFile::clean_filename(&format!("{} {}", id, &note.title))
			} else {
				NoteFile::clean_filename(&note.title)
			};
			if note.file.stem.to_lowercase() != new_filename.to_lowercase() {
				fs.push((note.get_meta(), new_filename));
			}
		}
		fs
	}

	pub fn rename_note(&self, note_meta: &NoteMeta, new_stem: &str) -> io::Result<()> {
		let note = &self.notes[&WikiLink::FileName(note_meta.stem.to_string())];

		// Rename note file and replace NoteFile object in Note
		let new_note_file = note.borrow().file.rename(&new_stem)?;
		note.borrow_mut().insert_file(new_note_file);

		self.update_filename_backlinks_to(&note_meta.stem, &new_stem)?;

		Ok(())
	}

	fn update_filename_backlinks_to(
		&self,
		old_file_stem: &str,
		new_file_stem: &str,
	) -> io::Result<()> {
		let old_filename_link = WikiLink::FileName(old_file_stem.to_owned());

		if self.backlinks.contains_key(&old_filename_link) {
			let old_link = Note::get_wikilink(&None, &EMPTY_STRING, &old_file_stem);
			let new_link = Note::get_wikilink(&None, &EMPTY_STRING, &new_file_stem);

			for backlink in self.backlinks[&old_filename_link].iter() {
				{
					let new_note_file: NoteFile;
					{
						let new_contents =
							backlink.borrow().file.content.replace(&old_link, &new_link);
						new_note_file = backlink.borrow().file.replace_contents(&new_contents);
					}
					backlink.borrow_mut().insert_file(new_note_file);
				}
				backlink.borrow().save()?;
			}
		}

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use crate::note::*;

	fn get_default_parser() -> NoteParser {
		NoteParser::new(r"\d{11,14}", "**Links to this note**").expect("Test parser failed")
	}

	#[test]
	fn title_and_id_parser() {
		let parser = Rc::new(get_default_parser());
		let note = Note::new(
			NoteFile::new(&path::PathBuf::from(r"testdata/File Name Title.md")).unwrap(),
			Rc::clone(&parser),
		);

		assert_eq!(note.file.stem, "File Name Title");
		assert_eq!(note.id.unwrap(), "1234567890123");
		assert_eq!(note.title, "The Title In the Note Contents");
	}

	#[test]
	fn yaml1_title_parser() {
		let parser = Rc::new(get_default_parser());
		let note = Note::new(
			NoteFile::new(&path::PathBuf::from(r"testdata/yaml1.md")).unwrap(),
			Rc::clone(&parser),
		);

		assert_eq!(note.title, "Plain YAML title");
		assert!(note.id.is_none());
	}

	#[test]
	fn yaml2_title_parser() {
		let parser = Rc::new(get_default_parser());
		let note = Note::new(
			NoteFile::new(&path::PathBuf::from(r"testdata/yaml2.md")).unwrap(),
			Rc::clone(&parser),
		);

		assert_eq!(note.title, "Plein: YAML title");
		assert_eq!(note.id.unwrap(), "123123123123");
	}

	#[test]
	fn empty_file_parser() {
		let parser = Rc::new(get_default_parser());
		let note = Note::new(
			NoteFile::new(&path::PathBuf::from(r"testdata/Empty File With Name.md")).unwrap(),
			Rc::clone(&parser),
		);

		assert_eq!(note.file.stem, "Empty File With Name");
		assert_eq!(note.id, None);
		assert_eq!(note.title, "Empty File With Name");
	}

	#[test]
	fn title_parser() {
		let parser = Rc::new(get_default_parser());
		let note = Note::new(
			NoteFile::new(&path::PathBuf::from(r"testdata/12345678901 Test Note 1.md")).unwrap(),
			Rc::clone(&parser),
		);

		assert_eq!(note.title, "Test Note 1");
	}

	#[test]
	fn oneliner_parser() {
		let parser = Rc::new(get_default_parser());
		let note = Note::new(
			NoteFile::new(&path::PathBuf::from(r"testdata/One-liner.md")).unwrap(),
			Rc::clone(&parser),
		);

		assert_eq!(note.title, "Just a Heading");
	}

	#[test]
	fn task_parser() {
		let parser = Rc::new(get_default_parser());
		let note = Note::new(
			NoteFile::new(&path::PathBuf::from(r"testdata/Tasks.md")).unwrap(),
			Rc::clone(&parser),
		);

		assert!(note.tasks.contains(&"Don't forget to remember".to_string()));
		assert!(note.tasks.contains(&"Buy milk!".to_string()));
		assert!(note.tasks.contains(&"Nested".to_string()));
		assert!(note.tasks.contains(&"Tabbed with [[link]]".to_string()));
		assert!(note.tasks.contains(&"Final line".to_string()));
		assert_eq!(note.tasks.len(), 5);

		assert!(note
			.links
			.contains(&WikiLink::FileName(String::from("link"))));
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
			NoteFile::new(&path::PathBuf::from(r"testdata/BackLinks.md")).unwrap(),
			Rc::clone(&parser),
		);

		// All links in this file is in the backlinks section
		assert_eq!(note.links.len(), 0);

		// TODO: Add test, or are we done?
	}

	#[test]
	fn replace_backlinks() {
		let parser = Rc::new(get_default_parser());
		let note = Note::new(
			NoteFile::new(&path::PathBuf::from(r"testdata/BackLinks.md")).unwrap(),
			Rc::clone(&parser),
		);

		let c1 = note.get_contents_without_backlinks();
		assert_eq!(
			c1,
			"# Backlinks test case\r\n\r\nSome note text\r\n\r\n<!-- Here be dragons -->\r\n"
		);

		let c2 = note.get_backlinks_section_without_heading().unwrap();
		assert_eq!(
			c2.trim(),
			"- [[§An outline note]]\r\n- [[20201012145848]] Another note\r\n* Not a link"
		);

		let c3 =
			note.get_contents_with_new_backlinks("## Links to this note", "- [[The one and only]]");
		assert_eq!(c3, "# Backlinks test case\r\n\r\nSome note text\n\n## Links to this note\n\n- [[The one and only]]\n\n<!-- Here be dragons -->");
	}

	#[test]
	fn add_backlinks1() {
		let parser = Rc::new(get_default_parser());
		let note = Note::new(
			NoteFile::new(&path::PathBuf::from(r"testdata/One-liner.md")).unwrap(),
			Rc::clone(&parser),
		);

		let c1 = note.get_contents_without_backlinks();
		assert_eq!(c1, "# Just a Heading");

		assert!(note.get_backlinks_section_without_heading().is_none());

		let c3 = note.get_contents_with_new_backlinks(
			"## Links to this note",
			"- [[Link one]]\n- [[Link two]]",
		);
		assert_eq!(
			c3,
			"# Just a Heading\n\n## Links to this note\n\n- [[Link one]]\n- [[Link two]]"
		);
	}

	#[test]
	fn add_backlinks2() {
		let parser = Rc::new(get_default_parser());
		let note = Note::new(
			NoteFile::new(&path::PathBuf::from(r"testdata/12345678901 Test Note 1.md")).unwrap(),
			Rc::clone(&parser),
		);

		assert!(note.get_backlinks_section_without_heading().is_none());

		let c3 = note.get_contents_with_new_backlinks(
			"## Links to this note",
			"- [[Link one]]\n- [[Link two]]",
		);
		assert_eq!(
			c3,
			"This is the ID: 112233445566\n\n## Links to this note\n\n- [[Link one]]\n- [[Link two]]"
		);
	}

	#[test]
	fn clean_filename() {
		assert_eq!(
			NoteFile::clean_filename("Just a normal file name"),
			"Just a normal file name"
		);
		assert_eq!(
			NoteFile::clean_filename("<Is/this\\a::regular?*?*?file>"),
			"Is this a regular file"
		);
		assert_eq!(NoteFile::clean_filename(".hidden file"), "hidden file");
		assert_eq!(
			NoteFile::clean_filename("illegal in windows."),
			"illegal in windows"
		);
		assert_eq!(
			NoteFile::clean_filename("a . in the middle"),
			"a . in the middle"
		);
		assert_eq!(
			NoteFile::clean_filename("pipe | is also forbidden"),
			"pipe is also forbidden"
		);
		assert_eq!(
			NoteFile::clean_filename("Try some whitespace: \t\r\n--"),
			"Try some whitespace --"
		);
		assert_eq!(
			NoteFile::clean_filename("C# is a nice language!"),
			"C# is a nice language!"
		);
		assert_eq!(NoteFile::clean_filename(".:/?."), "");
	}

	#[test]
	fn file_encodings_utf8_bom() {
		let parser = Rc::new(get_default_parser());
		let note = Note::new(
			NoteFile::new(&path::PathBuf::from(r"testdata/BOM.md")).unwrap(),
			Rc::clone(&parser),
		);

		assert_eq!(note.file.content.chars().next().unwrap(), '\u{feff}');
	}

	#[test]
	fn file_encodings_win1252() {
		match NoteFile::new(&path::PathBuf::from(r"testdata/Win-1252.md")) {
			Ok(_) => panic!("Shouldn't be able to read Win-1252 file"),
			Err(_) => (),
		};
	}
}

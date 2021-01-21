mod innerm {
	use crate::ftree;
	use ansi_term::Colour;
	use lazy_static::*;
	use regex::Regex;
	use std::collections::HashMap;
	use std::collections::HashSet;
	use std::fmt;
	use std::hash::{Hash, Hasher};
	use std::rc::Rc;
	use std::{fs, io, path};

	lazy_static! {
		static ref EMPTY_STRING: String = String::from("");
		// These characters are replaced with " " (illegal in Windows)
		static ref ILLEGAL_FILE_CHARS: Regex = Regex::new("[<>:*?/\"\\\\]").unwrap();
		// "." at the beginning or end are removed
		static ref SURROUNDING_DOTS: Regex = Regex::new(r"(\A\.|\.\z)").unwrap();
		// Replace double spaces with single
		static ref DOUBLE_SPACES: Regex = Regex::new(r" +").unwrap();
	}

	#[derive(Debug)]
	pub struct NoteFile {
		path: String,
		stem: String,
		extension: String,
		content: String,
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

		/** Checks if filename begins or ends with a dot or space.
		 * This would be highly unorthodox file names, so we assume it's wrong. */
		fn begins_or_ends_with_dot_or_space(filename: &str) -> bool {
			if filename.len() == 0 {
				return false;
			}
			let first_char = filename.chars().nth(0).unwrap();
			let last_char = filename.chars().last().unwrap();
			first_char == ' ' || first_char == '.' || last_char == ' ' || last_char == '.'
		}

		pub fn save(path: &str, contents: &str) -> io::Result<()> {
			// Make sure file always ends with one newline
			fs::write(&path, String::from(contents.trim_end()) + "\n")
		}

		/** Renames file, assuming that the path is valid and escaped */
		pub fn rename(oldpath: &str, newpath: &str) -> io::Result<()> {
			fs::rename(oldpath, newpath)
		}
	}

	#[derive(Debug)]
	struct Note {
		file: NoteFile,
		title: String,
		title_lower: String,
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
			let title = Note::get_title_assoc(&file, &parser);
			Note {
				id: Note::get_id_assoc(&file, &parser),
				title_lower: title.to_lowercase(),
				title: title,
				links: Note::get_links_assoc(&file, &parser),
				todos: Note::get_todos_assoc(&file, &parser),
				parser,
				file,
			}
		}

		fn get_id_assoc(file: &NoteFile, parser: &NoteParser) -> Option<String> {
			// First try to get ID from note file name
			if let Some(id) = parser.get_id(&file.stem) {
				Some(id)
			// Then try to get ID from note contents
			} else if let Some(id) = parser.get_id(&file.content) {
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
			parser.get_wiki_links(parser.get_contents_without_backlinks(&file.content))
		}

		fn get_todos_assoc(file: &NoteFile, parser: &NoteParser) -> Vec<String> {
			parser.get_todos(&file.content)
		}

		fn get_contents_without_backlinks(&self) -> &str {
			self.parser
				.get_contents_without_backlinks(&self.file.content)
		}

		fn get_backlinks_section_without_heading(&self) -> Option<&str> {
			self.parser
				.get_backlinks_section_without_heading(&self.file.content)
		}

		fn has_outgoing_links(&self) -> bool {
			self.links.len() > 0
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

		fn get_wikilink_to(&self) -> String {
			Note::get_wikilink(&self.id, &self.title, &self.file.stem)
		}

		fn get_wikilink(id: &Option<String>, title: &String, file_stem: &String) -> String {
			// Link either to ID or filename
			let link_target: &String = if let Some(i) = id { i } else { file_stem };

			let link_desc: &String = if title == link_target {
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

	#[derive(Debug)]
	pub struct NoteParser {
		id_pattern: String,
		id_expr: Regex,
		wiki_link_expr: Regex,
		h1_expr: Regex,
		todo_expr: Regex,
		backlinks_heading: String,
		backlink_expr: Regex,
	}

	impl NoteParser {
		pub fn new(id_pattern: &str, backlinks_heading: &str) -> Result<NoteParser, &'static str> {
			let id_expr_str = format!(r"(?:\A|\s)({})(?:\z|\b)", &id_pattern);
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

		fn split_contents_at_backlinks_heading<'a>(
			&self,
			text: &'a str,
		) -> Option<(&'a str, &'a str)> {
			if let Some(i) = text.find(&self.backlinks_heading) {
				Some(text.split_at(i))
			} else {
				None
			}
		}

		fn get_contents_without_backlinks<'a>(&self, text: &'a str) -> &'a str {
			match self.split_contents_at_backlinks_heading(text) {
				Some((contents, _)) => contents.trim(),
				None => text,
			}
		}

		fn get_backlinks_section<'a>(&self, text: &'a str) -> Option<&'a str> {
			match self.split_contents_at_backlinks_heading(text) {
				Some((_, backlinks)) => Some(backlinks.trim()),
				None => None,
			}
		}

		fn get_backlinks_section_without_heading<'a>(&self, text: &'a str) -> Option<&'a str> {
			match self.split_contents_at_backlinks_heading(text) {
				Some((_, backlinks)) => Some(backlinks[self.backlinks_heading.len()..].trim()),
				None => None,
			}
		}

		fn get_backlinks<'a>(&self, text: &'a str) -> Vec<String> {
			// Note: backlinks should be unique, but
			let mut backlinks = Vec::new();
			if let Some(backlinks_section) = self.get_backlinks_section_without_heading(text) {
				for capture in self.backlink_expr.captures_iter(backlinks_section) {
					backlinks.push(capture[1].to_string());
				}
			}
			backlinks
		}
	}

	pub struct NoteCollection {
		/** Lookup for IDs and file names to all notes */
		notes: HashMap<WikiLink, Rc<Note>>,
		/** List of all notes */
		notes_iter: Vec<Rc<Note>>,
		backlinks: HashMap<WikiLink, Vec<Rc<Note>>>,
	}

	impl NoteCollection {
		pub fn collect_files(root: &path::Path, extension: &str, parser: NoteParser) -> NoteCollection {
			let parser = Rc::new(parser);
			let mut notes = HashMap::new();
			let mut notes_iter = Vec::new();
			let mut backlinks = HashMap::new();

			let note_paths = ftree::get_files(root, extension);

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

				let note = Rc::new(Note::new(note_file, Rc::clone(&parser)));
				notes_iter.push(Rc::clone(&note));

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

				for link in &note.links {
					// Ignore "backlinks" to self
					if !note.is_link_to(link) {
						backlinks
							.entry(link.clone())
							.or_insert(Vec::new())
							.push(Rc::clone(&note));
					}
				}
			}

			// TODO: Doesn't always want to sort by title, should probably be elsewhere
			notes_iter.sort_by(|a, b| a.title_lower.cmp(&b.title_lower));

			NoteCollection {
				notes,
				notes_iter,
				backlinks,
			}
		}

		fn visit_notes(&self, callback: &mut dyn FnMut(&Note)) {
			for note in &self.notes_iter {
				callback(&note);
			}
		}

		pub fn count(&self) -> usize {
			self.notes_iter.len()
		}

		pub fn count_with_id(&self) -> usize {
			let mut count: usize = 0;
			let mut f = |note: &Note| {
				if let Some(_) = &note.id {
					count += 1;
				}
			};
			self.visit_notes(&mut f);
			count
		}

		pub fn count_links(&self) -> usize {
			self.backlinks.len()
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

		fn get_incoming_links(&self, note: &Note) -> HashSet<Rc<Note>> {
			let empty: Vec<Rc<Note>> = Vec::new();
			let links1: HashSet<&Rc<Note>> = self
				.backlinks
				.get(&WikiLink::FileName(note.file.stem.to_string()))
				.unwrap_or(&empty)
				.iter()
				.collect();

			let links2: HashSet<&Rc<Note>> = if let Some(id) = &note.id {
				self.backlinks
					.get(&WikiLink::Id(id.to_string()))
					.unwrap_or(&empty)
					.iter()
					.collect()
			} else {
				HashSet::new()
			};

			links1.union(&links2).map(|rcn| Rc::clone(rcn)).collect()
		}

		/** Get notes with no incoming links, but at least one outgoing */
		pub fn get_sources(&self) -> Vec<NoteMeta> {
			let mut sources = Vec::new();
			let mut f = |note: &Note| {
				if note.has_outgoing_links() && !self.note_has_incoming_links(note) {
					sources.push(note.get_meta());
				}
			};
			self.visit_notes(&mut f);
			sources
		}

		/** Get notes with no outgoing links, but at least one incoming */
		pub fn get_sinks(&self) -> Vec<NoteMeta> {
			let mut sinks = Vec::new();
			let mut f = |note: &Note| {
				if !note.has_outgoing_links() && self.note_has_incoming_links(note) {
					sinks.push(note.get_meta());
				}
			};
			self.visit_notes(&mut f);
			sinks
		}

		/** Get notes with no incoming or outgoing links */
		pub fn get_isolated(&self) -> Vec<NoteMeta> {
			let mut isolated = Vec::new();
			let mut f = |note: &Note| {
				if !note.has_outgoing_links() && !self.note_has_incoming_links(note) {
					isolated.push(note.get_meta());
				}
			};
			self.visit_notes(&mut f);
			isolated
		}

		pub fn get_broken_links(&self) -> Vec<(&WikiLink, Vec<NoteMeta>)> {
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

		pub fn get_todos(&self) -> Vec<(NoteMeta, Vec<String>)> {
			let mut todos = Vec::new();
			let mut f = |note: &Note| {
				if note.todos.len() > 0 {
					todos.push((note.get_meta(), note.todos.clone()));
				}
			};
			self.visit_notes(&mut f);
			todos
		}

		pub fn remove_backlinks(&self) -> i32 {
			let mut count = 0;
			let mut f = |note: &Note| {
				if let Some((content, _)) = note
					.parser
					.split_contents_at_backlinks_heading(&note.file.content)
				{
					if let Err(e) = NoteFile::save(&note.file.path, &content) {
						eprintln!("Error while saving note file {}: {}", note.file.path, e);
					} else {
						count += 1;
					}
				}
			};
			self.visit_notes(&mut f);
			count
		}

		pub fn update_backlinks(&self) -> i32 {
			let mut count = 0;
			let mut f = |note: &Note| {
				let mut incoming_links: Vec<Rc<Note>> =
					self.get_incoming_links(note).into_iter().collect();

				// First sort by filename to get a stable sort when titles are identical
				incoming_links.sort_by(|a, b| a.file.stem.cmp(&b.file.stem));
				incoming_links.sort_by(|a, b| a.title_lower.cmp(&b.title_lower));

				let new_backlinks: Vec<String> = incoming_links
					.iter()
					.map(|linking_note| "- ".to_string() + &linking_note.get_wikilink_to())
					.collect();
				let new_section = new_backlinks.join("\n");

				let current_section = note
					.get_backlinks_section_without_heading()
					.unwrap_or_default();
				if current_section != new_section {
					let new_contents = if new_section.len() > 0 {
						// Add or update backlinks
						note.get_contents_without_backlinks().trim_end().to_string()
							+ "\n\n" + &note.parser.backlinks_heading
							+ "\n\n" + &new_section
					} else {
						// Remove backlinks
						note.get_contents_without_backlinks().to_string()
					};
					if let Err(e) = NoteFile::save(&note.file.path, &new_contents) {
						eprintln!("Error while saving note file {}: {}", note.file.path, e);
					} else {
						count += 1;
					}
				}
			};
			self.visit_notes(&mut f);
			count
		}

		pub fn get_mismatched_filenames(&self) -> Vec<(NoteMeta, String)> {
			let mut fs = Vec::new();
			let mut f = |note: &Note| {
				let new_filename = if let Some(id) = &note.id {
					NoteFile::clean_filename(&format!("{} {}", id, &note.title))
				} else {
					NoteFile::clean_filename(&note.title)
				};
				if note.file.stem.to_lowercase() != new_filename.to_lowercase() {
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

	#[cfg(test)]
	mod tests {
		use crate::innerm::*;

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
				NoteFile::new(&path::PathBuf::from(r"testdata/File Name Title.md")).unwrap(),
				Rc::clone(&parser),
			);

			assert_eq!(note.file.stem, "File Name Title");
			assert_eq!(note.id.unwrap(), "1234567890123");
			assert_eq!(note.title, "The Title In the Note Contents");
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
			let parser = get_default_parser();
			let note_file =
				NoteFile::new(&path::PathBuf::from(r"testdata/12345678901 Test Note 1.md"))
					.unwrap();
			let title = Note::get_title_assoc(&note_file, &parser);
			assert_eq!(title, "Test Note 1");
		}

		#[test]
		fn oneliner_parser() {
			let parser = get_default_parser();
			let note_file = NoteFile::new(&path::PathBuf::from(r"testdata/One-liner.md")).unwrap();
			let title = Note::get_title_assoc(&note_file, &parser);
			assert_eq!(title, "Just a Heading");
		}

		#[test]
		fn link_parser() {
			let parser = Rc::new(get_default_parser());
			let note = Note::new(
				NoteFile::new(&path::PathBuf::from(r"testdata/Links.md")).unwrap(),
				Rc::clone(&parser),
			);

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
				NoteFile::new(&path::PathBuf::from(r"testdata/Todos.md")).unwrap(),
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
				NoteFile::new(&path::PathBuf::from(r"testdata/Backlinks.md")).unwrap(),
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
		fn backlinks_heading_parser() {
			let parser = Rc::new(get_default_parser());
			let note = Note::new(
				NoteFile::new(&path::PathBuf::from(r"testdata/Backlinks.md")).unwrap(),
				Rc::clone(&parser),
			);

			if let Some((c1, c2)) = parser.split_contents_at_backlinks_heading(&note.file.content) {
				assert_eq!(c1, "# Backlinks test case\r\n\r\nSome note text\r\n\r\n");
				assert!(c2.starts_with(&parser.backlinks_heading));
			} else {
				panic!("split_contents_at_backlinks_heading() failed");
			}

			if let Some(s) = parser.get_backlinks_section_without_heading(&note.file.content) {
				assert!(s.starts_with("- [["));
			} else {
				panic!("get_backlinks_section_without_heading() failed");
			}

			if let Some(_) =
				parser.split_contents_at_backlinks_heading(&"# Just a heading\r\nNo backlinks ...")
			{
				panic!("split_contents_at_backlinks_heading() failed");
			}

			if let Some(_) = parser
				.get_backlinks_section_without_heading(&"# Just a heading\r\nNo backlinks ...")
			{
				panic!("get_backlinks_section_without_heading() failed");
			}
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
			assert_eq!(NoteFile::clean_filename(".:/?."), "");
		}

		#[test]
		fn file_encodings_utf8_bom() {
			let parser = Rc::new(get_default_parser());
			let note = Note::new(
				NoteFile::new(&path::PathBuf::from(r"testdata/BOM.md")).unwrap(),
				Rc::clone(&parser),
			);

			// Vad är konsekvensen av att BOM är kvar? Den kommer ju då skrivas tillbaka korrekt när anteckningen sparas. Uppstår något problem?

			assert_eq!(
				note.file.content,
				"Yxmördaren Julia Blomqvist på fäktning i Schweiz"
			);
		}

		#[test]
		fn file_encodings_win1252() {
			let parser = Rc::new(get_default_parser());
			let note = Note::new(
				NoteFile::new(&path::PathBuf::from(r"testdata/Win-1252.md")).unwrap(),
				Rc::clone(&parser),
			);
			assert_eq!(
				note.file.content,
				"Yxmördaren Julia Blomqvist på fäktning i Schweiz"
			);
		}
	}
}

mod ftree {
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
						let path = err.path().unwrap_or(path::Path::new("")).display();
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
			.map(|s| s.starts_with("."))
			.unwrap_or(false)
	}
}

use crate::innerm::*;
use chrono::Utc;
use debug_print::debug_println;
use std::error::Error;
use std::fs;

#[derive(Debug)]
pub struct Config {
	pub id_pattern: String,
	pub backlinks_heading: String,
	pub extension: String,
	pub path: String,
	pub command: String,
}

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
	let start_time = Utc::now();
	let notes = NoteCollection::collect_files(
		&fs::canonicalize(&config.path)?,
		&config.extension,
		NoteParser::new(&config.id_pattern, &config.backlinks_heading)?,
	);
	let duration_collect_files = Utc::now() - start_time;

	let start_time = Utc::now();
	match config.command.as_str() {
		"list-broken-links" => print_broken_links(&notes),
		"list-sources" => print_sources(&notes),
		"list-sinks" => print_sinks(&notes),
		"list-isolated" => print_isolated(&notes),
		"list-todos" => print_todos(&notes),
		"remove-backlinks" => remove_backlinks(&notes),
		"update-backlinks" => update_backlinks(&notes),
		"update-filenames" => update_filenames(&notes)?,
		_ => print_stats(&notes),
	}
	let duration_subcommand = Utc::now() - start_time;

	debug_println!(
		"NoteCollection::collect_files() took {} ms",
		duration_collect_files.num_milliseconds()
	);
	debug_println!(
		"Subcommand took {} ms",
		duration_subcommand.num_milliseconds()
	);

	Ok(())
}

fn print_stats(note_collection: &NoteCollection) {
	println!("# Statistics");

	println!("- Found number of notes: {}", note_collection.count());
	println!(
		"- Found number of note IDs: {}",
		note_collection.count_with_id()
	);
	println!("- Found number of links: {}", note_collection.count_links());
}

fn print_todos(note_collection: &NoteCollection) {
	println!("# To-do");

	for (note, todos) in note_collection.get_todos() {
		println!("\n## {}\n", note.get_wikilink_to());

		for todo in todos {
			println!("- [ ] {}", todo);
		}
	}
}

fn print_sources(note_collection: &NoteCollection) {
	let notes = note_collection.get_sources();

	println!("# Source notes");
	println!(
		"{} notes have no incoming links, but at least one outgoing link\n",
		notes.len()
	);
	print_note_wikilink_list(&notes);
}

fn print_sinks(note_collection: &NoteCollection) {
	let notes = note_collection.get_sinks();

	println!("# Sink notes");
	println!(
		"{} notes have no outgoing links, but at least one incoming link\n",
		notes.len()
	);
	print_note_wikilink_list(&notes);
}

fn print_isolated(note_collection: &NoteCollection) {
	let notes = note_collection.get_isolated();

	println!("# Isolated notes");
	println!("{} notes have no incoming or outgoing links\n", notes.len());
	print_note_wikilink_list(&notes);
}

fn print_note_wikilink_list(notes: &Vec<NoteMeta>) {
	for note in notes {
		println!("- {}", note.get_wikilink_to());
	}
}

fn print_broken_links(note_collection: &NoteCollection) {
	let broken_links = note_collection.get_broken_links();

	println!("# Broken links");

	for (link, notes) in broken_links {
		let linkers: Vec<String> = notes.iter().map(|n| n.get_wikilink_to()).collect();
		println!("- \"{}\" links to unknown {}", linkers.join(" and "), link);
	}
}

fn remove_backlinks(note_collection: &NoteCollection) {
	let num_removed = note_collection.remove_backlinks();
	println!("Removed backlinks section from {} notes", num_removed);
}

fn update_backlinks(note_collection: &NoteCollection) {
	let num_updated = note_collection.update_backlinks();
	println!("Updated backlinks section in {} notes", num_updated);
}

fn update_filenames(note_collection: &NoteCollection) -> Result<(), Box<dyn Error>> {
	for (note, new_filename) in note_collection.get_mismatched_filenames() {
		let original_file_name = format!("{}.{}", note.stem, note.extension);
		let reply = rprompt::prompt_reply_stdout(&format!(
			"Rename \"{}\" to \"{}\"? ([y]/n) ",
			original_file_name, new_filename
		))?;

		if reply == "y" || reply == "" {
			if note.path.ends_with(&original_file_name) {
				let folder = &note.path[..(note.path.len() - original_file_name.len())];
				let new_path = folder.to_string() + &new_filename;
				NoteFile::rename(&note.path, &new_path)?;
			} else {
				// TODO: Return as Err
				eprintln!("Error: probably a bug in how the file name path is determined");
			}
		}
	}

	Ok(())
}

use crate::innerm::{NoteFile, WikiLink};
use lazy_static::*;
use regex::Regex;

lazy_static! {
	static ref YAML_TITLE_EXPR: Regex =
		Regex::new(r#"\A\s*['"]?title['"]?\s*: \s*['"]?([^'"]+)['"]?\z"#).unwrap();
	static ref WIKILINK_EXPR: Regex = Regex::new(
		&"\\[\\[(?x)
			# Label can occur first, ends with |
			([^\\[\\]]+\\|)?
			(
				# Filename or ID
				{:link_chars:}+?
			)
			# Section can occur last, starts with #
			((?-x:#)[^\\[\\]]+)?
			\\]\\]"
			.replace("{:link_chars:}", "[^<>:*?/\\]\\[\"\\\\\\t]")
	)
	.unwrap();
	static ref TASK_EXPR: Regex = Regex::new(r"\A\s*[-+*]\s+\[ \]\s+(.+?)\z").unwrap();
	static ref BACKLINK_EXPR: Regex = Regex::new(r"\A[-+*]\s*(.*?)\z").unwrap();
	static ref INDENTED_LIST_EXPR: Regex = Regex::new(r"\A\s+[-+*]\s.+\z").unwrap();
	// Two ways to start and end code blocks
	static ref CODEBLOCK_TOKEN_1: &'static str = "```";
	static ref CODEBLOCK_TOKEN_2: &'static str = "~~~";
}

#[derive(Debug)]
enum ParseState<'a> {
	Initial,
	Yaml,
	Regular,
	CodeBlock(&'a str),
	BackLinks,
}

#[derive(Debug)]
pub struct NoteData {
	pub titles: Vec<String>,
	pub ids: Vec<String>,
	pub links: Vec<WikiLink>,
	pub tasks: Vec<String>,
	pub backlinks_start: Option<usize>,
	pub backlinks_end: Option<usize>,
}

#[derive(Debug)]
pub struct NoteParser {
	id_pattern: String,
	id_expr: Regex,
	pub backlinks_heading: String,
}

impl NoteParser {
	pub fn new(id_pattern: &str, backlinks_heading: &str) -> Result<NoteParser, &'static str> {
		let id_expr_str = format!(r"(?:\A|\s)({})(?:\z|\b)", &id_pattern);
		let id_expr = match Regex::new(&id_expr_str) {
			Ok(expr) => expr,
			Err(_) => return Err("Cannot parse ID format as regular expression"),
		};

		// Replace whitespace character representations
		let backlinks_heading = backlinks_heading.to_string();

		Ok(NoteParser {
			id_pattern: id_pattern.to_string(),
			id_expr,
			backlinks_heading,
		})
	}

	pub fn parse(&self, text: &str) -> NoteData {
		let mut titles = Vec::new();
		let mut ids = Vec::new();
		let mut links = Vec::new();
		let mut tasks = Vec::new();
		let mut backlinks_start: Option<usize> = None;
		let mut backlinks_end: Option<usize> = None;

		let mut state = ParseState::Initial;
		let mut start_end = find_first_line(&text, starts_with_bom(&text));

		loop {
			if start_end.is_none() {
				break;
			}

			let start = start_end.unwrap().0;
			let end = start_end.unwrap().1;
			let ln = &text[start..end];
			let ln_bytes = ln.as_bytes();

			match state {
				ParseState::Initial => {
					if ln.starts_with("---") {
						state = ParseState::Yaml;
					} else {
						// Parse the line again in another state
						state = ParseState::Regular;
						continue;
					}
				}
				ParseState::Yaml => {
					if ln.starts_with("---") || ln.starts_with("...") {
						state = ParseState::Regular;
					} else if ln_bytes[0] == b'#' {
						// Ignore comments
					} else {
						if ln.len() > 7 {
							if let Some(capture) = YAML_TITLE_EXPR.captures(ln) {
								titles.push(capture[1].to_owned());
							}
						}
						if let Some(capture) = self.id_expr.captures(ln) {
							ids.push(capture[1].to_owned());
						}
						if ln.len() > 4 && ln.contains("[[") {
							if let Some(wl) = self.get_wiki_links(ln) {
								links.extend(wl);
							}
						}
					}
				}
				ParseState::Regular => {
					// Heading 1
					if ln_bytes.len() > 2 && ln_bytes[0] == b'#' && ln_bytes[1] == b' ' {
						titles.push(
							// Remove {.attributes} and trailing # characters and spaces
							// See https://pandoc.org/MANUAL.html#pandocs-markdown
							NoteParser::strip_heading_attributes(&ln[2..])
								.trim_end_matches(|c| c == ' ' || c == '#')
								.to_owned(),
						);
						if let Some(capture) = self.id_expr.captures(ln) {
							ids.push(capture[1].to_owned());
						}
						if ln.len() > 4 && ln.contains("[[") {
							if let Some(wl) = self.get_wiki_links(ln) {
								links.extend(wl);
							}
						}
					} else if (ln_bytes[0] == b'\t' || ln.starts_with("    "))
						&& !INDENTED_LIST_EXPR.is_match(ln)
					{
						// Ignore code blocks (not indented list items) and line-breaks
					} else if ln.starts_with(*CODEBLOCK_TOKEN_1)
						|| ln.starts_with(*CODEBLOCK_TOKEN_2)
					{
						state = ParseState::CodeBlock(&ln[..3]);
					} else if ln == self.backlinks_heading {
						backlinks_start = Some(start);
						state = ParseState::BackLinks;
					} else {
						if let Some(capture) = self.id_expr.captures(ln) {
							ids.push(capture[1].to_owned());
						}
						if ln.len() > 4 && ln.contains('[') {
							if let Some(wl) = self.get_wiki_links(ln) {
								links.extend(wl);
							}
							if let Some(capture) = TASK_EXPR.captures(ln) {
								tasks.push(capture[1].to_string());
							}
						}
					}
				}
				ParseState::CodeBlock(token) => {
					if ln.starts_with(token) {
						// Found end token
						state = ParseState::Regular;
					}
				}
				ParseState::BackLinks => {
					if !BACKLINK_EXPR.is_match(ln) {
						// Backlinks list had ended, something else is here
						backlinks_end = Some(start);

						// Parse the line again in another state
						state = ParseState::Regular;
						continue;
					}
				}
			}

			// Parse the next line
			start_end = find_next_line(&text, end);
		}

		NoteData {
			titles,
			ids,
			links,
			tasks,
			backlinks_start,
			backlinks_end,
		}
	}

	/// Remove Pandoc-style attributes at the end of a heading ("{#id}")
	pub fn strip_heading_attributes(text: &str) -> &str {
		if text.as_bytes()[text.len() - 1] == b'}' {
			if let Some(start) = text.rfind('{') {
				return &text[..start];
			}
		}
		text
	}

	pub fn get_id(&self, text: &str) -> Option<String> {
		match self.id_expr.captures(&text) {
			None => None,
			Some(capture) => Some(capture[1].to_string()),
		}
	}

	#[inline]
	fn is_id(&self, text: &str) -> bool {
		self.id_expr.is_match(text)
	}

	pub fn remove_id(&self, text: &str) -> String {
		self.id_expr.replace(text, "").trim().to_owned()
	}

	pub fn get_wiki_links(&self, text: &str) -> Option<Vec<WikiLink>> {
		let mut captures = WIKILINK_EXPR.captures_iter(&text).peekable();
		if captures.peek().is_none() {
			return None;
		}
		let mut links = Vec::new();
		for capture in captures {
			let link = capture[2].to_string();
			if self.is_id(&link) {
				links.push(WikiLink::Id(link));
			} else if !NoteFile::begins_or_ends_with_dot_or_space(&link) {
				links.push(WikiLink::FileName(link));
			}
		}
		Some(links)
	}
}

/// Returns the size of the BOM if it exists
fn starts_with_bom(text: &str) -> usize {
	if text.len() >= 3 && text.chars().next().unwrap() == '\u{feff}' {
		3
	} else {
		0
	}
}

fn find_newline(text: &str, offset: usize) -> Option<usize> {
	let mut pos = offset;
	for char in text[offset..].bytes() {
		if char == b'\n' || char == b'\r' {
			return Some(pos);
		}
		pos += 1;
	}
	None
}

/// Find byte position (start, end) of first line, or None
fn find_first_line(text: &str, offset: usize) -> Option<(usize, usize)> {
	let mut pos = offset;
	for char in text[offset..].chars() {
		if char == '\n' || char == '\r' {
			pos += 1;
		} else {
			match find_newline(&text, pos) {
				None => return Some((pos, text.len())),
				Some(pos_next_newline) => {
					return Some((pos, pos_next_newline));
				}
			}
		}
	}
	None
}

/// Find byte position (start, end) of next line, or None
fn find_next_line(text: &str, offset: usize) -> Option<(usize, usize)> {
	match find_newline(&text, offset) {
		None => None,
		Some(pos) => find_first_line(&text, pos),
	}
}

#[cfg(test)]
mod tests {
	use crate::mdparse;
	use crate::mdparse::{NoteParser, WikiLink};
	use std::fs;

	#[test]
	fn trim_bom() {
		let with_bom = fs::read_to_string(r"testdata/Markdown1.md").unwrap();

		assert!(with_bom.starts_with('\u{feff}'));
		assert_eq!(mdparse::starts_with_bom(&with_bom), 3);

		assert_eq!(mdparse::starts_with_bom(&"Hello, world!"), 0);
		assert_eq!(mdparse::starts_with_bom(&"."), 0);
	}

	#[test]
	fn find_first_line() {
		assert_eq!(mdparse::find_first_line("", 0), None);
		assert_eq!(mdparse::find_first_line("\r", 0), None);
		assert_eq!(mdparse::find_first_line("\n", 0), None);

		let text = "Lorem ipsum dolor sit amet";
		let (s1, e1) = mdparse::find_first_line(text, 0).unwrap();
		assert_eq!(s1, 0);
		assert_eq!(&text[s1..e1], "Lorem ipsum dolor sit amet");

		let text = "Lorem ipsum dolor sit amet\r\n";
		let (s1, e1) = mdparse::find_first_line(text, 0).unwrap();
		assert_eq!(s1, 0);
		assert_eq!(&text[s1..e1], "Lorem ipsum dolor sit amet");

		let text = "\r\r\n\nLorem ipsum dolor sit amet";
		let (s1, e1) = mdparse::find_first_line(text, 0).unwrap();
		assert_eq!(s1, 4);
		assert_eq!(&text[s1..e1], "Lorem ipsum dolor sit amet");

		let text = "\rLorem\ripsum\ndolor\r\nsit\namet\r\n";
		let (s1, e1) = mdparse::find_first_line(text, 0).unwrap();
		assert_eq!(s1, 1);
		assert_eq!(&text[s1..e1], "Lorem");

		let text = "🔥";
		let (s1, e1) = mdparse::find_first_line(text, 0).unwrap();
		assert_eq!(s1, 0);
		assert_eq!(&text[s1..e1], "🔥");
	}

	#[test]
	fn find_next_line() {
		assert_eq!(mdparse::find_next_line("", 0), None);
		assert_eq!(mdparse::find_next_line("\r", 0), None);
		assert_eq!(mdparse::find_next_line("\n", 0), None);
		assert_eq!(
			mdparse::find_next_line("Lorem ipsum dolor sit amet", 0),
			None
		);
		assert_eq!(mdparse::find_next_line("🔥", 0), None);

		let text = "\rLorem\ripsum\ndolor\r\nsit\namet\r\n";
		let (s1, e1) = mdparse::find_next_line(text, 0).unwrap();
		println!("1. {}..{} = {}", s1, e1, &text[s1..e1]);
		assert_eq!(s1, 1);
		assert_eq!(&text[s1..e1], "Lorem");

		let (s1, e1) = mdparse::find_next_line(&text, s1).unwrap();
		assert_eq!(s1, 7);
		assert_eq!(&text[s1..e1], "ipsum");

		let (s1, e1) = mdparse::find_next_line(&text, s1).unwrap();
		assert_eq!(s1, 13);
		assert_eq!(&text[s1..e1], "dolor");

		let text = "🔥\n🔥\n";
		let (s1, e1) = mdparse::find_next_line(text, 0).unwrap();
		assert_eq!(s1, 5);
		assert_eq!(&text[s1..e1], "🔥");
	}

	#[test]
	fn strip_attributes() {
		assert_eq!(
			NoteParser::strip_heading_attributes("My heading"),
			"My heading"
		);
		assert_eq!(
			NoteParser::strip_heading_attributes("My heading {#foo}"),
			"My heading "
		);
		assert_eq!(
			NoteParser::strip_heading_attributes("My {heading} {#foo}"),
			"My {heading} "
		);

		// Only remove {} at the end!
		assert_eq!(
			NoteParser::strip_heading_attributes("{-} My heading"),
			"{-} My heading"
		);

		assert_eq!(
			NoteParser::strip_heading_attributes("My }{ heading"),
			"My }{ heading"
		);
	}

	#[test]
	fn parse_md1() {
		let text = fs::read_to_string(r"testdata/Markdown1.md").unwrap();
		let parser = NoteParser::new(
			r"\d{12,14}",
			"## Links to this note {#backlinks .unnumbered}",
		)
		.unwrap();
		let data = parser.parse(&text);

		let expected_ids = [
			"123123123123",
			"1111111111110",
			"1234567891011",
			"1212121212121",
			"9900021212121",
		];

		assert_eq!(data.ids.len(), expected_ids.len());
		for (expected, actual) in expected_ids.iter().zip(data.ids.iter()) {
			assert_eq!(actual, expected);
		}

		let expected_titles = [
			"Markdown: A markup language",
			"Markdown Test File",
			"This is a heading inside a comment",
			"Then another heading",
		];

		assert_eq!(data.titles.len(), expected_titles.len());
		for (expected, actual) in expected_titles.iter().zip(data.titles.iter()) {
			assert_eq!(actual, expected);
		}

		let expected_links = [
			WikiLink::FileName("Related note1".to_owned()),
			WikiLink::FileName("Related note2".to_owned()),
			WikiLink::Id("1234567891011".to_owned()),
			WikiLink::FileName("should this count as a link".to_owned()),
			WikiLink::FileName("One last link".to_owned()),
		];

		assert_eq!(data.links.len(), expected_links.len());
		for (expected, actual) in expected_links.iter().zip(data.links.iter()) {
			assert_eq!(actual, expected);
		}
	}
}

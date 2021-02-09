use crate::note::WikiLink;
use lazy_static::*;
use regex::Regex;
use std::borrow::Cow;

lazy_static! {
	static ref YAML_TITLE_EXPR: Regex =
		Regex::new(r#"\A\s*['"]?title['"]?\s*: \s*['"]?([^'"]+)['"]?\z"#).unwrap();

	static ref LINK_CHARS: &'static str = "[^<>:*?|/\\]\\[\"\\\\\\t]";
	static ref WIKILINK_SIMPLE_EXPR: Regex = Regex::new(
		&"\\[\\[(.+?)\\]\\]"
		.replace("{:link_chars:}", *LINK_CHARS)
	)
	.unwrap();
	static ref TASK_EXPR: Regex = Regex::new(r"\A\s*[-+*]\s+\[ \]\s+(.+?)\z").unwrap();
	static ref BACKLINK_EXPR: Regex = Regex::new(r"\A[-+*]\s*(.*?)\z").unwrap();
	static ref INDENTED_LIST_EXPR: Regex = Regex::new(r"\A\s+([-+*]|\d+\.)\s.+\z").unwrap();

	/// Characters that can be escaped in Markdown
	static ref ESCAPED_CHARS_EXPR: Regex = Regex::new(r"\\([\\`\*_{}\[\]<>()#+-\.!|])").unwrap();

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
							// Remove {.attributes} and trailing spaces
							// See https://pandoc.org/MANUAL.html#pandocs-markdown
							escape_markdown(
								NoteParser::strip_heading_attributes(&ln[2..]).trim_end(),
							)
							.to_string(),
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
		let mut captures = WIKILINK_SIMPLE_EXPR.captures_iter(&text).peekable();
		if captures.peek().is_none() {
			return None;
		}
		let mut links = Vec::new();
		for capture in captures {
			let link = capture[1].to_string();
			if self.is_id(&link) {
				links.push(WikiLink::Id(link));
			} else {
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

fn escape_markdown(text: &str) -> Cow<str> {
	ESCAPED_CHARS_EXPR.replace_all(text, "$1")
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

	#[test]
	fn parse_links() {
		let text = fs::read_to_string(r"testdata/Links.md").unwrap();
		let parser = NoteParser::new(r"\d{11,14}", "**Links to this note**").unwrap();
		let data = parser.parse(&text);

		let expected_links = vec![
			WikiLink::Id("20210104073402".to_owned()),
			WikiLink::Id("20210103212011".to_owned()),
			WikiLink::FileName("Filename Link".to_owned()),
			WikiLink::FileName("Search Query Link".to_owned()),
			WikiLink::FileName("Regular Link To Wiki URI".to_owned()),
			WikiLink::FileName("Org-Mode Link Text][Org-Mode Link".to_owned()),
			WikiLink::FileName("using labelled links|labelling wiki links".to_owned()),
			WikiLink::FileName("the filename first#section then".to_owned()),
			WikiLink::FileName("my [not so pretty] link".to_owned()),
			WikiLink::FileName(" some text and then [[a link".to_owned()),
		];

		for expected_link in &expected_links {
			assert!(data.links.contains(expected_link));
		}

		let unexpected_links = vec![
			WikiLink::FileName("Inside Fenced Code Block".to_owned()),
			WikiLink::FileName("Also fenced".to_owned()),
		];

		for unexpected_link in &unexpected_links {
			assert!(!data.links.contains(unexpected_link));
		}

		assert_eq!(data.links.len(), expected_links.len());
	}

	#[test]
	fn oneliner_parser() {
		let text = r"# Just a heading \#";
		let parser = NoteParser::new(r"\d{14}", "## Links to this note").unwrap();
		let data = parser.parse(&text);

		assert!(data.titles.contains(&"Just a heading #".to_owned()));
		assert_eq!(data.titles.len(), 1);
		assert_eq!(data.links.len(), 0);
		assert_eq!(data.ids.len(), 0);
		assert_eq!(data.tasks.len(), 0);
		assert!(data.backlinks_start.is_none());
		assert!(data.backlinks_end.is_none());
	}

	#[test]
	fn escaped_characters() {
		assert_eq!(
			mdparse::escape_markdown(r"C\#\! \{ 0 \+\- 1 \}"),
			"C#! { 0 +- 1 }"
		);
		assert_eq!(
			mdparse::escape_markdown(r"Escape\.\`\(\\\[\|\*\]\)"),
			r"Escape.`(\[|*])"
		);
	}
}

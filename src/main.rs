use clap::{crate_version, App, Arg, SubCommand};
use noteexplorer::{run, Config};
use std::process;

fn main() {
	let matches = App::new("NoteExplorer")
		.version(crate_version!())
		.arg(
			Arg::with_name("extension")
				.short("e")
				.long("extension")
				.help("File extension of note files")
				.takes_value(true)
				.value_name("ext")
				.default_value("md"),
		)
		.arg(
			Arg::with_name("id_format")
				.short("i")
				.long("idfmt")
				.help("Regular expression pattern for note ID:s")
				.takes_value(true)
				.value_name("format")
				.default_value("\\d{14}"),
		)
		.arg(
			Arg::with_name("backlinks_heading")
				.short("b")
				.long("backhead")
				.help("Heading to insert before backlinks")
				.takes_value(true)
				.value_name("format")
				.default_value("-----------------\\r\\n**Links to this note**"),
		)
		.arg(
			Arg::with_name("path")
				.help("Path to the note files directory")
				.default_value(".")
				.index(1),
		)
		.subcommand(
			SubCommand::with_name("list-broken-links")
				.alias("brokenlinks")
				.about("Prints a list of broken links"),
		)
		.subcommand(
			SubCommand::with_name("list-orphans")
				.alias("orphans")
				.about("Prints a list of notes with no incoming links"),
		)
		.subcommand(
			SubCommand::with_name("list-todos")
				.aliases(&["todo", "todos"])
				.about("Prints a list of TODOs"),
		)
		.subcommand(
			SubCommand::with_name("update-backlinks")
				.alias("backlinks")
				.about("Updates backlinks sections in all notes"),
		)
		.subcommand(
			SubCommand::with_name("update-filenames")
				.alias("rename")
				.about("Updates note filenames with ID and title"),
		)
		.get_matches();

	// TODO: Add "exclude-files" argument
	// TODO: Create new note from template
	// TODO: Get list of longest notes
	// TODO: Get list of shortest notes

	let command = matches.subcommand_name().unwrap_or_default();

	let config = Config {
		extension: matches.value_of("extension").unwrap().to_string(),
		id_pattern: matches.value_of("id_format").unwrap().to_string(),
		backlinks_heading: matches.value_of("backlinks_heading").unwrap().to_string(),
		path: matches.value_of("path").unwrap().to_string(),
		command: command.to_string(),
	};

	if let Err(e) = run(config) {
		eprintln!("Application error: {}", e);
		process::exit(1);
	}
}

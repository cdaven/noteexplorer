use chrono::Utc;
use clap::{crate_version, App, Arg, SubCommand};
use debug_print::debug_println;
use noteexplorer::{run, Config};
use std::process;

fn main() {
	let matches = App::new("NoteExplorer")
		.version(crate_version!())
		.author("Christian Davén <christian@daven.se>")
		.about("Helps organizing your stack of linked Markdown notes")
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
				.long("id-format")
				.help("Regular expression pattern for note ID:s")
				.takes_value(true)
				.value_name("format")
				.default_value("\\d{14}"),
		)
		.arg(
			Arg::with_name("backlinks_heading")
				.short("b")
				.long("backlinks-heading")
				.help("Heading to insert before backlinks")
				.takes_value(true)
				.value_name("format")
				.default_value("## Links to this note"),
		)
		.arg(
			Arg::with_name("PATH")
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
			SubCommand::with_name("list-isolated")
				.alias("isolated")
				.about("Prints a list of notes with no incoming or outgoing links"),
		)
		.subcommand(
			SubCommand::with_name("list-sinks")
				.alias("sinks")
				.about("Prints a list of notes with no outgoing links"),
		)
		.subcommand(
			SubCommand::with_name("list-sources")
				.alias("sources")
				.about("Prints a list of notes with no incoming links"),
		)
		.subcommand(
			SubCommand::with_name("list-tasks")
				.aliases(&["tasks", "todos"])
				.about("Prints a list of tasks"),
		)
		.subcommand(
			SubCommand::with_name("update-backlinks")
				.alias("backlinks")
				.about("Updates backlink sections in all notes"),
		)
		.subcommand(
			SubCommand::with_name("remove-backlinks")
				.about("Removes backlink sections in all notes"),
		)
		.subcommand(
			SubCommand::with_name("update-filenames")
				.alias("rename")
				.about("Updates note filenames with ID and title")
				.arg(
					Arg::with_name("force")
						.short("f")
						.help("Always update names, never prompt"),
				),
		)
		.get_matches();

	let command = matches.subcommand_name().unwrap_or_default();
	let mut force = false;
	if let Some(c) = matches.subcommand_matches("update-filenames") {
	 	force = c.is_present("force");
	}

	let config = Config {
		extension: matches.value_of("extension").unwrap().to_string(),
		id_pattern: matches.value_of("id_format").unwrap().to_string(),
		backlinks_heading: matches.value_of("backlinks_heading").unwrap().to_string(),
		path: matches.value_of("PATH").unwrap().to_string(),
		command: command.to_string(),
		force
	};

	let start_time = Utc::now();
	if let Err(e) = run(config) {
		eprintln!("Application error: {}", e);
		process::exit(1);
	}
	let duration = Utc::now() - start_time;
	debug_println!("run() took {} ms", duration.num_milliseconds());
}

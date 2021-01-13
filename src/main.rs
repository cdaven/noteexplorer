use clap::{crate_version, App, Arg, SubCommand};
use std::error::Error;
use std::process;

use noteexplorer::{run, Config};

// Anmäl VAB till Försäkringskassan
// Skriv upp VAB i Fortnox

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
				.required(true)
				.index(1),
		)
		// .subcommand(
		// 	SubCommand::with_name("test")
		// )
		.get_matches();

	let config = Config {
		extension: matches.value_of("extension").unwrap().to_string(),
		id_pattern: matches.value_of("id_format").unwrap().to_string(),
		backlinks_heading: matches.value_of("backlinks_heading").unwrap().to_string(),
		path: matches.value_of("path").unwrap().to_string(),
	};

	println!("{:?}", config);

	run(config);
}

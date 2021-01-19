# README

NoteExplorer is a tool to help organizing your stack of (wiki-)linked Markdown notes.

## General tips

When trying out NoteExplorer, please make backups of your notes, in case the tool doesn't work as expected, or you change your mind.

I keep all my notes in a Git repository, and commit them before trying something new. That way, I can always revert unwanted changes.

## Usage

```
    noteexplorer.exe [OPTIONS] [PATH] [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -b, --backlinks-heading <format>    Heading to insert before backlinks [default: ...]
    -e, --extension <ext>               File extension of note files [default: md]
    -i, --id-format <format>            Regular expression pattern for note ID:s [default: \d{14}]

ARGS:
    <PATH>    Path to the note files directory [default: .]

SUBCOMMANDS:
    help                 Prints this message or the help of the given subcommand(s)
    list-broken-links    Prints a list of broken links
    list-isolated        Prints a list of notes with no incoming or outgoing links
    list-sinks           Prints a list of notes with no outgoing links
    list-sources         Prints a list of notes with no incoming links
    list-todos           Prints a list of TODOs
    remove-backlinks     Removes backlink sections in all notes
    update-backlinks     Updates backlink sections in all notes
    update-filenames     Updates note filenames with ID and title
```

### Options

#### Extension

```sh
--extension "md"
```

Alias: `-e`

The extension of the files that should be read.

#### ID Format

```sh
--id-format "\d{14}"
```

Alias: `-i`

A regular expression for finding note ID:s in text and filenames. Read more below.

#### Backlinks heading

```sh
--backlinks-heading "## Links to this note"
```

Alias: `-b`

The heading that is expected or will be inserted before backlinks in notes. You can use \r, \n and \t characters in the string. If you do use newlines in your heading, be aware that newlines on Windows can be represented either as \r\n or just \n, and you will not see the difference.

Everything from this heading on, will be ignored by NoteExplorer when reading. So this affects the amount of words and links in the notes.

You should not write anything after this heading in your notes, as it will be removed when updating backlinks.

Note that in order to change this heading for existing notes, you must first run the subcommand `remove-backlinks` and specify the current heading. Otherwise, you will get double backlink sections!

### Subcommands

#### list-broken-links

Alias: `broken`

Lists all broken links from all notes. A broken link is one that NoteExplorer cannot resolve.

#### list-isolated

Alias: `isolated`

Lists all isolated notes, meaning notes with no incoming or outgoing links.

(The term is from graph theory.)

#### list-sinks

Alias: `sinks`

Lists all sink notes, meaning notes with no outgoing links, but at least one incoming link.

(The term is from graph theory.)

#### list-sources

Alias: `sources`

Lists all source notes, meaning notes with no incoming links, but at least one outgoing link.

(The term is from graph theory.)

#### list-todos

Alias: `todos`

Lists all open todos from all notes. A todo is a list item that starts with `- [ ] `. When you tick the box (`[x]`), the item will not show up the next time you list todos.

See e.g. [TODO.md](https://github.com/todo-md/todo-md).

#### remove-backlinks

Removes backlinks from all notes, using the heading from the `--backlinks-heading` argument.

Note that everything after the backlinks heading will be removed.

#### update-backlinks

Alias: `backlinks`

Updates backlinks in notes, using the heading from the `--backlinks-heading` argument.

Note that everything after the backlinks heading will be removed.

#### update-filenames

Alias: `rename`

Updates filenames of notes, based on the template `(<id>) <title>.<extension>`. So, if the ID is 12345678901234 and the H1 title in the note is "Title of the note", the filename will become "12345678901234 Title of the note.md".

If there is no ID, the filename will be just the title.

Some characters are illegal in Windows filenames, and will be replaced by spaces. Filenames in Windows can also not end with a period (".").

Asks for confirmation for each rename.

## Installation

For now, the only way to install is to copy the published binaries (Windows and Linux). See [releases on Github](https://github.com/cdaven/noteexplorer/releases).

## Limitations/rules

NoteExplorer is in many ways inspired by [the ideas behind Zettlr](https://docs.zettlr.com/en/academic/zkn-method/), but should work with other note-taking systems that support Markdown wikilinks.

You can create links between notes with this syntax: `[[<label>|<id or filename>#<section>]]`, where both label and section are optional. There are two ways to pinpoint other notes in links.

### Filename links

You can link to the target note's filename, like `[[There and back again]]`. Note that the path and file extension are omitted. Your note-taking application is assumed to find the right file anyway.

This means that filenames must be unique in a collection of notes. Also, filename links are expected to follow the same rules as filenames in Windows:

Filename links:

- Cannot contain any of the characters `<`, `>`, `:`, `"`, `/`, `\`, `|`, `?`, `*`, tabs and newlines. Most of these are ok on Linux and Mac OS, and some on Windows, but this helps make sure your notes and this application is cross-platform.
- Cannot contain any of the characters `[`, `]`, `#`. This is simply because it messes up the wikilink syntax.
- Cannot start or end with a dot (`.`). Technically, Windows only forbids trailing dots, but leading dots means "hidden file" in Linux and Mac OS.
- Cannot start or end with a space (` `). This isn't forbidden by the operating systems, but seems like a reasonable limitation to avoid false positives.

The labels and sections can contain any character except for `[`, `]`, `#`, `|`.

### ID links

The idea behind the Zettelkasten ID is to allow the filename to change without having to update all links pointing to that file. The ID can be included in the filename or the note itself.

Of course, NoteExplorer must know the format of your IDs to be able to detect them. The default value is 14 digits (in regular expressions speak, this is "\d{14}"), which is a timestamp like 20210119212027. You can specify another format with the `--id-format` option.

ID links are simple enough: `[[20210119212027]]`. Just the ID, no filename or title.

### Specifying the ID of a note

When specifying a note's ID, you can put it anywhere in the filename or note contents.

If there is a string mathing your ID format in the filename, it is assumed to be the note's ID. Otherwise, the note's contents is scanned for the first matching ID.

In both cases, the ID must have either a space or nothing in front of it, and a "non-word" character or nothing after it. This makes sure we don't match URLs with `/` and phone numbers with `+` before long numbers. It's not perfect, so it's best to keep the ID in the filename.

### Parsing note titles

NoteExplorer tries to parse the note's titles. The first Markdown H1 heading (such as `# An Unexpected Journey`) is preferred. Otherwise, the note's filename, after removing the ID, is used as a title.

### Traversing your note collection

The `PATH` given to NoteExplorer is the directory folder, and all subdirectories will be traversed, looking for notes. However, all files and directories that begin with a dot (`.`) are ignored, since they are by tradition hidden in Linux and Mac OS.

Yet another limitation: NoteExplorer can only read files in UTF-8.

## Background

This tool was inspired by [Andy Matuschak's note-link-janitor](https://github.com/andymatuschak/note-link-janitor/). The first few iterations were done in Python during the Autumn and Winter of 2020. This became [katalorg](https://github.com/cdaven/katalorg).

When I grew too frustrated with Python, I looked for a new language, and decided to learn Rust. It was hard at first, but is clearly a great choice for such command-line tools.

The first version of NoteExplorer was ready in January 2021.

## Developing and building

NoteExplorer is written in Rust, and can be built on (at least) Windows, Mac OS and Linux.

First, [install Rust](https://www.rust-lang.org/tools/install) and all its dependencies.

Then you can build NoteExplorer by simply running `cargo build` or `cargo build --release`.

Run unit tests with `cargo test`.

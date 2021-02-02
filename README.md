# README

NoteExplorer is a CLI tool to help organizing your stack of (wiki-)linked Markdown notes.

Features:

- Adds or updates a "backlinks" section in notes
- Network analysis of links
- Collects tasks scattered in notes
- Reveals broken links
- Updates filenames based on ID and title

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
    list-tasks           Prints a list of tasks
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

The heading that is expected or will be inserted before backlinks in notes.

Unfortunately, it seems that you cannot use "--" as part of the heading, since the arguments parser will think this is another option.

Note that in order to change this heading for existing notes, you must first run the subcommand `remove-backlinks` and specify the current heading. Otherwise, you will get multiple backlink sections!

### Subcommands

Note that all subcommands that explore connections between notes ignore links from the backlinks section, since these should not be considered outgoing links. To make sure this works, you have to include the `--backlinks-heading` option for these subcommands as well.

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

#### list-tasks

Alias: `tasks`

Lists all open tasks from all notes. A task is a list item that starts with `- [ ] `. When you tick the box (`[x]`), the item will not show up in the list anymore.

To "delete" tasks without removing them from the note, use the regular strikethrough/deleted syntax: `- ~~[ ] Give ring to Frodo~~`

Note that tasks in numbered lists are not included.

#### remove-backlinks

Removes backlinks from all notes, using the heading from the `--backlinks-heading` argument.

#### update-backlinks

Alias: `backlinks`

Updates backlinks in notes, following the heading from the `--backlinks-heading` argument.

Read more about [Backlink sections in notes](https://github.com/cdaven/noteexplorer/wiki/Backlinks-sections-in-notes)

#### update-filenames

Alias: `rename`

Updates filenames of notes, based on the template `(<id>) <title>.<extension>`. So, if the ID is 20210119212027 and the title in the note is "There and back again", the filename will become "20210119212027 There and back again.md". (The extension is set with the `--extension` option.)

If there is no ID, the filename will be just the title. If there is not title, the filename will be just the ID.

Read more about how NoteExplorer picks the [title](https://github.com/cdaven/noteexplorer/wiki/How-NoteExplorer-picks-the-title-of-a-note) and ID of a note.

Some invalid characters will be cleaned from the title before saving as a file, since the operating systems object to them. Read more about this below, in "Filename links".

Asks for confirmation for each rename.

## Installation

For now, binaries exist for Windows and Linux, and can be [downloaded from GitHub](https://github.com/cdaven/noteexplorer/releases).

I suggest you put the executable in the root folder of your notes, and keep a terminal window open. Maybe create some shell scripts with your preferred arguments.

You can also install the Windows version [with Chocolatey](https://chocolatey.org/packages/noteexplorer/):

```sh
choco install noteexplorer
```

## Limitations/rules

NoteExplorer is in many ways inspired by [the ideas behind Zettlr](https://docs.zettlr.com/en/academic/zkn-method/), but should work with other note-taking systems that support Markdown wikilinks.

### Filename links

You can link to the target note's filename, like `[[The Hobbit]]`. Note that filename links are case insensitive and the path and file extension is omitted. Your note-taking application is assumed to find the right file anyway.

From version 0.3.0, illegal filename characters and `[` and `]` are allowed in filename links. This makes it easier to find invalid links with the `list-broken-links` subcommand.

### ID links

The idea behind the Zettelkasten ID is to allow the filename to change without having to update all links pointing to that file. The ID can be included in the filename or the note itself.

Of course, NoteExplorer must know the format of your IDs to be able to detect them. The default value is 14 digits (in regular expressions speak, this is "\d{14}"), which is a timestamp like 20210119212027. You can specify another format with the `--id-format` option.

ID links are simple enough: `[[20210119212027]]`. Just the ID, no filename or title.

### Specifying the ID of a note

It's highly recommended to put the note's ID either in the filename, the YAML frontmatter, or at the top of the note.

You cannot put the ID in indented or fenced code blocks, in nested list items or in the backlinks section.

If there is a string mathing your ID format in the filename, it is assumed to be the note's ID. Otherwise, the note's contents is scanned for the first matching ID.

In both cases, the ID must have either a space or nothing in front of it, and a "non-word" character or nothing after it. This makes sure we don't match URLs with `/` and phone numbers with `+` before long numbers.

Note that if you specify the note's ID as a link: `[[20210119212027]]`, it will be considered a link to another note, and not the note's own ID.

### Parsing note titles

[How NoteExplorer picks the title of a note](https://github.com/cdaven/noteexplorer/wiki/How-NoteExplorer-picks-the-title-of-a-note)

### Traversing your note collection

The `PATH` given to NoteExplorer is the root directory. All subdirectories will be traversed, looking for notes. However, all files and directories that begin with a dot (`.`) are ignored, since they are by tradition hidden in Linux and Mac OS.

If two or more notes use the same ID, you will get a warning.

Yet another limitation: NoteExplorer can only read files in UTF-8, with or without byte order mark (BOM).

### Limitations to the Markdown parser

The Markdown parser is simple, and will not honor HTML comments.

If you use styling in the headings (e.g. **bold** or _italic_), it will not be stripped.

## Background

This tool was inspired by [Andy Matuschak's note-link-janitor](https://github.com/andymatuschak/note-link-janitor/).

Read more in [Why is NoteExplorer written in Rust?](https://github.com/cdaven/noteexplorer/wiki/Why-is-NoteExplorer-written-in-Rust%3F)

## Developing and building

NoteExplorer is written in Rust, and can be built on (at least) Windows, Mac OS and Linux.

First, [install Rust](https://www.rust-lang.org/tools/install) and all its dependencies.

Then you can build NoteExplorer by simply running `cargo build` or `cargo build --release`.

- Run unit tests with `cargo test`.
- Lint code with `cargo clippy`.
- Profile code with `cargo profiler callgrind`.

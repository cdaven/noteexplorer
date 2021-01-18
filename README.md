# README

NoteExplorer is a tool to help organizing your stack of (wiki-)linked Markdown notes.

## General tips

When trying out NoteExplorer, please make backups of your notes, in case the tool doesn't work as expected, or you change your mind.

I keep all my notes in a Git repository, and commit them before trying something new. That way, I can always revert unwanted changes.

## Usage

```
NoteExplorer 0.1.0

USAGE:
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

The heading that is expected or will be inserted before backlinks in notes. You can use \r, \n and \t characters in the string.

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

Lists all open todos from all notes. A todo is a list item that starts with `- [ ] `. When you tick the box (`[x]`), the item will no longer be listed by the subcommand.

See e.g. [TODO.md](https://github.com/todo-md/todo-md).

#### remove-backlinks (not implemented yet)


#### update-backlinks (not implemented yet)

Alias: `backlinks`

Updates backlinks in notes, using the heading from the `--backlinks-heading` argument.

Note that everything after the backlinks heading will be removed.

#### update-filenames

```sh
noteexplorer [PATH] update-filenames
```

Alias: `rename`

Updates filenames of notes, based on the template `id title.<extension>`. So, if the ID is 12345678901234 and the H1 title in the note is "Title of the note", the filename will become "12345678901234 Title of the note.md".

If there is no ID, the filename will be just the title.

Some characters are illegal in Windows filenames, and will be replaced by spaces. Filenames in Windows can also not end with a period (".").

Asks for confirmation for each rename.

## Limitations/rules

NoteExplorer is in many ways inspired by [the ideas behind Zettlr](https://docs.zettlr.com/en/academic/zkn-method/), but should work with other Zettelkasten-like note-taking systems.

### Note ID:s

- Note ID:s are expected to be numerical, by default with 14 digits
- The format can be specified with the `--id-format` option
- An ID in text must be surrounded by whitespace or the beginning or end of a line
- Notes are expected to contain at most one ID. Subsequent ID:s will be ignored.
- ID:s are expected to be unique across all folders in a collection

### Wiki links

- Wiki links follow this format: `[[<label>|<id or filename>#<section>]]`
  - Label and section are optional and will be ignored for now
  - The filename is without path and extension
- The label and section cannot contain the characters `[` or `]`
- Wiki links cannot contain any of the characters <, >, :, ", /, \, |, ?, * -- ?

### Files

- File names (without the extension) are expected to be unique across all folders in a collection
- Files and directories that begin with a dot (".") are ignored
- Files are expected to comply
- Files are expected to use the UTF-8 encoding

## Where ID and title are expected

The note contents has precedence over the file name. So, if you put one ID or title in the contents
and another in the file name, NoteExplorer picks the ones from the contents.

Here's why:

1. When you create a note, you can't come up with a great title before you've written anything. So
   the file name is probably preliminary at this point.
2. It's (usually) easier to change the heading in the note than to rename the file.
3. Your file system doesn't accept all characters in file names, which means that the file name will
   be a somewhat poorer representation of the real title.

## Background

This tool was inspired by [Andy Matuschak's note-link-janitor](https://github.com/andymatuschak/note-link-janitor/). The first few iterations were done in Python during the Autumn and Winter of 2020. This became [katalorg](https://github.com/cdaven/katalorg).

When I grew too frustrated with Python, I looked for a new language, and decided to learn Rust. It was hard at first, but is clearly a great choice for such command-line tools.

The first version of NoteExplorer was ready in January 2021.

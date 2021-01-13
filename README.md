# README

## Limitations/rules

### ID:s

- Note ID:s are expected to be numerical, by default with 14 digits
- An ID must be surrounded by whitespace or the beginning or end of a line
- Notes are expected to contain at most one ID. Subsequent ID:s will be ignored.

### Wiki links

- Wiki links start with `[[` end end with `]]`
- Wiki links can contain the ID of a note
- Wiki links can contain the file name of a note (no path and no extension)
- Wiki links cannot contain any of the letters `[` or `]`

### Files

- The default file name extension is `.md`
- File names (without the extension) are expected to be unique across all folders in a collection
- Files are expected to use the UTF-8 encoding
- Files and directories that begin with a dot (".") are ignored

## Where ID and title are expected

The note contents has precedence over the file name. So, if you put one ID or title in the contents
and another in the file name, `katalorg` picks the ones from the contents.

Here's why:

1. When you create a note, you can't come up with a great title before you've written anything. So
   the file name is probably preliminary at this point.
2. It's (usually) easier to change the heading in the note than to rename the file.
3. Your file system doesn't accept all characters in file names, which means that the file name will
   be a somewhat poorer representation of the real title.

# Changelog


## Release 0.3.0 ?

- Drop support for labels and sections in links (`[[label|filename#section]]`)
- Allow all characters in links, to make all invalid links findable with `list-broken-links`
- Added missing pipe character (`|`) to list of illegal filename characters

## Release 0.2.1. - January 31, 2021

- Fix bug where parser missed links in nested ordered lists

## Release 0.2.0 - January 29, 2021

- New Markdown parser that supports YAML front matter and ignores code blocks
- Searching for note files is much faster
- Removed feature to have backlinks heading include newlines and tabs
- Allow editing notes after backlinks section, and not overwrite it
- The `list-todos` subcommand is renamed `list-tasks`

## Release 0.1.1 - January 19, 2021

A few minor bugfixes:

- Don't count links to self as incoming/backlinks
- Don't list backlinks more than once per linking page
- When updating filenames, accept case differences
- Allow non-word characters after ID

## Release 0.1.0 - January 19, 2021

The very first release for Windows and Linux.

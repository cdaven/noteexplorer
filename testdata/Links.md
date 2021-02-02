---
- related: [[20210104073402]], parsed in Yaml now
...

- [ ] check
- [[]]
- Indented listst should also work
    + [[Filename Link]]
        1. [[Search Query Link]]
        2. [[20210103212011]]
- [Link Text]([[Regular Link To Wiki URI]])
- [[Org-Mode Link Text][Org-Mode Link]]

## Advanced syntax

NoteExplorer has chosen not to support [[using labelled links|labelling wiki links]] and linking straight to a section: [[the filename first#section then]].

However, filenames and links should probably be able to contain [] chars: [[my [not so pretty] link]]

This should not become a link: [[ some text and then [[a link]] and then some ]].

## Code

```javascript
var s = "[[Inside Fenced Code Block]]"
```

	[[Also fenced]]

## Newline

Links spanning newlines should not be accepted: [[no
way]]

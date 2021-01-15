---
- related: [[20210104073402]]
...

- [ ] check
- [[]]
- [[Filename Link]]
- [[Search Query Link]]
- [[20210103212011]]
- [Link Text]([[Regular Link To Wiki URI]])
- [[Org-Mode Link Text][Org-Mode Link]]

## Invalid/irregular file names

This shouldn't be a link: [[...]], am I right?

This shouldn't be a link: [[?]], am I right?

This shouldn't be a link: [[*/\*]], am I right?

This shouldn't be a link: [[ surrounded by spaces ]], am I right?

## Advanced syntax

Some editors have support for [[using labelled links|labelling wiki links]] and linking straight to
a section: [[the filename first#section then]]. Maybe you can even combine the two: [[a label|a note#a section]]?

This could get caught in a greedy regex: [[ and then ]] | and some # ]].

## Code

```javascript
var s = "[[Inside Fenced Code Block]]"
```

## Newline

Links spanning newlines should not be accepted: [[no
way]]

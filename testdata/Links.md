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

The labels can of course contain more characters: [[*Stars*and/or<stripes>?|Stars and stripes]]

And why not sections as well?: [[Stars or stripes#one/two\three.]]

Maybe multiple links can trigger some greedy regex? [[label 123|link 123]] [[label 234|link 234]]

## Code

```javascript
var s = "[[Inside Fenced Code Block]]"
```

## Newline

Links spanning newlines should not be accepted: [[no
way]]

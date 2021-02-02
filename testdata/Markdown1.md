
---
# title: no title
	"title"  : 'Markdown: A markup language'
id: 123123123123
seealso: [[Related note1]], [[Related note2]]
...

#atag #an-other #the_third

1111111111110

# Markdown Test File {#h1}

Lorem ipsum dolor sit amet, consectetur adipiscing elit. Praesent facilisis efficitur turpis, eget porta purus sodales eu. In hac habitasse platea dictumst. Vestibulum quis massa vitae ex vehicula eleifend. Etiam non ligula dapibus, tristique tellus id, euismod elit. Nulla urna purus, eleifend sit amet nisl vel, commodo imperdiet leo. In hac habitasse platea dictumst. Nullam eget lacus libero. Proin suscipit ut ante ac pellentesque.

<p>I really like using Markdown.</p>

That is so funny! 😂

\# Not a heading, since it was escaped

Empty heading:

#

> #### The quarterly results look great!
>
> - Revenue was off the chart.
> - Profits were higher than ever.
>
>  *Everything* is going according to **plan**.

![Tux, the Linux mascot](/assets/images/tux.png)

Setext-style headings are not supported
===================================

## Lines, 1234567891011

___

---

***

## Lists [[1234567891011]]

1. First item
2. Second item
3. Third item
4. Fourth item

+ First item
+ Second item
+ Third item
+ Fourth item

\* Without the backslash, this would be a bullet in an unordered list.

## YAML

Ignore YAML inside the file:

---
title: Bogus
---

Etiam quis massa pharetra, feugiat arcu id, scelerisque sem. Morbi vel augue quis arcu scelerisque egestas. Aliquam finibus elit leo. Mauris non lacinia neque, et rutrum lectus. Cras nisl mi, sollicitudin id hendrerit ut, scelerisque nec sem. Suspendisse rutrum eros tortor, eu vulputate orci luctus ut. Cras vitae faucibus elit, in molestie ipsum. Proin dictum sed justo quis condimentum. Nam a luctus eros. Nulla auctor congue nunc. Nam eu blandit lectus. Nunc lectus ex, varius vel lorem sed, pellentesque semper ex. Vestibulum tempor elit tellus, vitae aliquet dui varius eget.

## Comments

<!--Let me comment this:
# This is a heading inside a comment
And some text as well-->

You can also use comments inline: <!-- [[should this count as a link]]? ID Should count: 1212121212121 -->

## Markdown links

I love supporting the **[EFF](https://eff.org)**.
This is the *[Markdown Guide](https://www.markdownguide.org)*.
See the section on [`code`](#code).

In a hole in the ground there lived a hobbit. Not a nasty, dirty, wet hole, filled with the ends
of worms and an oozy smell, nor yet a dry, bare, sandy hole with nothing in it to sit down on or to
eat: it was a [hobbit-hole][1], and that means comfort.

[1]: <https://en.wikipedia.org/wiki/Hobbit#Lifestyle> "Hobbit lifestyles"

Here's a simple footnote[^1]

[^1]: This is the first footnote.

## Code

Indented with tabs:

	#[derive(Debug)]
	pub struct NoteData {
		0000021212121,
		pub ids: Vec<String>,
		pub links: Vec<WikiLink>,
		pub tasks: Vec<String>,
		pub backlinks: Vec<String>,
	}

Indented with spaces:

    #[derive(Debug)]
        0000021212121,
        pub struct NoteData {
        pub titles: Vec<String>,
        pub ids: Vec<String>,
        pub links: Vec<WikiLink>,
        pub tasks: Vec<String>,
        pub backlinks: Vec<String>,
    }

Fenced blocks:

```rust
#[derive(Debug)]
	0000021212121,
	pub struct NoteData {
	pub titles: Vec<String>,
	pub ids: Vec<String>,
	pub links: Vec<WikiLink>,
	pub tasks: Vec<String>,
	pub backlinks: Vec<String>,
}
```

~~~rust
#[derive(Debug)]
	0000021212121,
	pub struct NoteData {
	pub titles: Vec<String>,
	pub ids: Vec<String>,
	pub links: Vec<WikiLink>,
	pub tasks: Vec<String>,
	pub backlinks: Vec<String>,
}
~~~

Inline code: ``Use `code` in your Markdown file.``

## Definition lists

First Term
: This is the definition of the first term.

Second Term
: This is one definition of the second term.
: This is another definition of the second term.

## Tables

| Syntax      | Description |
| ----------- | ----------- |
| Header      | Title       |
| Paragraph   | Text        |

## Pandoc-supported HTML identifiers {#identifier .class .class key=value key=value}

Write something about this in a wiki page with a link to https://pandoc.org/MANUAL.html#pandocs-markdown

## Links to this note {#backlinks .unnumbered}

- [[There could be a link here]]
- [[And here as well]]

<!-- Should this be allowed? -->

# Then another heading

Etiam accumsan mi sit amet erat varius vulputate. Morbi erat sapien, eleifend blandit orci sit amet, convallis condimentum diam. Sed vel commodo risus, in bibendum ante. Ut consequat sit amet dolor vel porta. Quisque dignissim lorem eleifend dui ultrices malesuada nec vel libero. Pellentesque euismod, nisl finibus pellentesque tempus, massa velit euismod ipsum, id egestas nulla nisi a sem. Nunc varius nibh eget turpis aliquam imperdiet. Aenean viverra tortor id cursus efficitur. Aliquam molestie pharetra vulputate. Suspendisse nec pharetra augue, volutpat eleifend risus. Mauris vel ante at neque dapibus accumsan sit amet sed leo. Vestibulum nec semper orci. Nam varius dui quis risus mollis, porta placerat massa efficitur.

[[One last link]]

9900021212121
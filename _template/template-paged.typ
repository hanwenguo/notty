#import "site.typ"
#import "transclusion-paged.typ": _sequence, _styled, transclude

#let _no-numbering = sys.inputs.at("no-numbering", default: none) != none

#let sans-fonts = ("IBM Plex Sans", "IBM Plex Sans SC")
#let serif-fonts = ("IBM Plex Serif", "FZShuSong-Z01")
#let serif-italic-fonts = (
  "IBM Plex Serif",
  "Zhuque Fangsong (technical preview)",
)

#let _metadata(date, identifier, ..attrs) = {
  let author = attrs.at("author", default: none)
  [#block(width: 100%, [
    #set text(font: sans-fonts, size: 11pt)
    #if author != none { author }
    #if author != none and date != none { sym.dot.c }
    #if date != none { date.display("[month repr:long] [day], [year]") }
    #if author != none or date != none { v(0.5em) }
  ]) <metadata>]
}

#let _main-part(
  content,
  title: none,
  date: none,
  identifier: none,
  author: none,
  taxon: "",
  lang: site.config.lang,
) = {
  heading(depth: 1, {
    if taxon != none { taxon }
    context if counter("transclusion-depth").get().at(0) != 0 {
      counter(heading).step(level: counter("transclusion-depth").get().at(0))
    }
    context if counter(heading).get().at(0) != 0 and not _no-numbering {
      if taxon != none { " " } + counter(heading).display() + ". "
    } else if taxon != none { ". " }
    title
  })
  _metadata(date, identifier, author: author)
  content
}

#let handle-denote-link(it) = {
  let dest = it.dest
  if type(dest) == str and dest.starts-with("denote:") {
    let identifier = dest.slice(7)
    let new-dest = site.config.base-url + "pdf/" + identifier + ".pdf"
    underline(stroke: (dash: "dotted"), link(new-dest, it.body))
  } else {
    underline(it)
  }
}

#let template(
  title: "",
  date: none,
  identifier: none,
  author: none,
  taxon: none,
  lang: site.config.lang,
  tags: none,
) = doc => {
  set page(
    paper: "us-letter",
    margin: (
      left: 1in,
      right: 1in,
      top: 1.5in,
      bottom: 1.5in,
    ),
    footer: context {
      set text(font: sans-fonts, size: 8pt)
      block(width: 100% + 3.5in - 1in, {
        if counter(page).get().first() != 1 {
          linebreak()
          [#counter(page).display()]
        }
      })
    },
  )

  set text(
    font: serif-fonts,
    fill: luma(30),
    style: "normal",
    weight: "regular",
    hyphenate: true,
    size: 11pt,
  )
  show text.where(style: "italic"): set text(font: serif-italic-fonts)

  set math.equation(numbering: "(1)")
  show math.equation: set block(spacing: 0.65em)
  show math.equation: set text(font: "IBM Plex Math")

  set enum(indent: 1em, body-indent: 1em)
  show enum: set par(justify: false)
  set list(indent: 1em, body-indent: 1em)
  show list: set par(justify: false)

  // set heading(numbering: if not _no-numbering { "1." } else { none })
  show heading.where(depth: 1): it => {
    let title = it.body
    {
      set text(hyphenate: false, size: 20pt, font: sans-fonts)
      set par(justify: false, leading: 0.2em, first-line-indent: 0pt)
      title
    }
  }

  show heading.where(depth: 2): it => {
    v(2em, weak: true)
    text(size: 14pt, weight: "bold", it)
    v(1em, weak: true)
  }

  show heading.where(depth: 3): it => {
    v(1.3em, weak: true)
    text(size: 13pt, weight: "regular", style: "italic", it)
    v(1em, weak: true)
  }

  show heading.where(depth: 4): it => {
    v(1em, weak: true)
    text(size: 11pt, style: "italic", weight: "thin", it)
    v(0.65em, weak: true)
  }

  set par(leading: 0.65em, first-line-indent: 1em, spacing: 0.65em)

  show link: handle-denote-link

  (
    [#metadata((
        taxon: taxon,
        title: title,
        author: author,
        date: date,
      )) <frontmatter>]
  )
  _main-part(
    doc,
    title: title,
    date: date,
    identifier: identifier,
    author: author,
    taxon: taxon,
    lang: lang,
  )
}

#let _find-main-content(c) = {
  let children = c.children
  let p = children.position(it => it.func() == _styled)
  if p != none {
    let child = children.at(p)
    _sequence(child.child.children)
  } else {
    none
  }
}

#let ln(dest, body) = link(dest, body)
// {
//   if type(dest) == str {
//     if dest.starts-with("denote:") {
//       let identifier = dest.slice(7)
//       let new-dest = site.config.base-url + "pdf/" + identifier + ".pdf"
//       link(new-dest, it.body)
//     }
//   } else { it }
// }

#let tr(url, hide-metadata: true, open: true) = {
  if url.starts-with("denote:") {
    let identifier = url.slice(7)
    let path = site.id-to-path(identifier)
    if hide-metadata {
      show label("metadata"): it => none
      transclude(
        path,
        heading-offset: 1,
        transform-heading: x => x,
        transform-other: x => {
          if x.func() == metadata { none } else { x }
        },
        find-main-content: _find-main-content,
      )
    } else {
      show label("metadata"): it => it
      transclude(
        path,
        heading-offset: 1,
        transform-heading: x => x,
        transform-other: x => {
          if x.func() == metadata { none } else { x }
        },
        find-main-content: _find-main-content,
      )
    }
  }
}

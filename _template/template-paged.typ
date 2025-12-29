#import "site.typ"

#let sans-fonts = ("Inter", "IBM Plex Sans", "IBM Plex Sans SC")
#let serif-fonts = ("Libertinus Serif", "IBM Plex Serif")

#let _default-metadata(date, identifier, ..attrs) = {
  let author = attrs.at("author", default: site.config.default-author.name)
  [#block(width: 100%, [
    #set text(font: sans-fonts, size: 11pt)
    #if author != none { author }
    #if author != none and date != none { sym.dot.c }
    #if date != none { date.display("[month repr:long] [day], [year]") }
    #if author != none or date != none { v(0.5em) }
  ]) <metadata>]
}

#let _metadata(date, identifier, ..attrs) = {
  let taxon = attrs.at("taxon", default: none)
  if taxon != none {
    let f = site.paged-metadata-taxon-map.at(taxon, default: _default-metadata)
    f(date, identifier, ..attrs)
  } else {
    _default-metadata(date, identifier, ..attrs)
  }
}

#let _main-part(
  content,
  title: none,
  date: none,
  identifier: none,
  ..attrs,
) = {
  let taxon = attrs.at("taxon", default: none)
  let author = attrs.at("author", default: site.config.default-author.name)
  heading(depth: 1, {
    if taxon != none {
      set text(style: "italic")
      taxon + ". "
    }
    title
  })
  _metadata(date, identifier, ..attrs)
  content
}

#let notty-section = _main-part

#let template-paged(
  title: "",
  date: none,
  identifier: none,
  ..attrs,
  // author: none,
  // taxon: none,
  // lang: site.config.lang,
) = doc => {
  set page(
    paper: "us-letter",
    margin: (
      left: 1.5in,
      right: 1.5in,
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

  set math.equation(numbering: "(1)")
  show math.equation: set block(spacing: 0.65em)
  show math.equation: set text(font: site.config.math-fonts)

  set enum(indent: 1em, body-indent: 1em)
  show enum: set par(justify: false)
  set list(indent: 1em, body-indent: 1em)
  show list: set par(justify: false)


  show heading.where(depth: 1): it => {
    let title = it.body
    {
      set text(hyphenate: false, size: 20pt, font: sans-fonts)
      set par(justify: false, leading: 0.2em, first-line-indent: 0pt)
      title
    }
  }

  show heading.where(depth: 2): it => {
    v(1.3em, weak: true)
    text(size: 14pt, weight: "bold", font: sans-fonts, it)
    v(1em, weak: true)
  }

  show heading.where(depth: 3): it => {
    v(1.3em, weak: true)
    text(size: 13pt, weight: "regular", font: sans-fonts, it)
    v(1em, weak: true)
  }

  show heading.where(depth: 4): it => {
    v(1em, weak: true)
    text(size: 11pt, weight: "light", font: sans-fonts, it)
    v(0.65em, weak: true)
  }

  set par(leading: 0.65em, first-line-indent: 0em, spacing: 1.3em)

  (
    [#metadata((
        title: title,
        date: date.display("[year repr:full][month repr:numerical][day]T[hour repr:24][minute][second]"),
        identifier: identifier,
      )) <frontmatter>]
  )
  _main-part(
    doc,
    title: title,
    date: date,
    identifier: identifier,
    ..attrs,
  )
}


#let ln-paged(dest, body) = link(dest, body)

// #let tr-paged(url, show-metadata: false, expanded: true) = par(link(url))
#let tr-paged(url, show-metadata: false, expanded: true, disable-numbering: false, demote-headings: true) = heading(depth: 2)[TRANSCUSION: #url]
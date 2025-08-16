#import "site.typ"
#import "transclusion-paged.typ": _sequence, _styled, transclude

#let _no-numbering = sys.inputs.at("no-numbering", default: none) != none

#let sans-fonts = site.config.pdf-sans-fonts
#let serif-fonts = site.config.pdf-serif-fonts

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
  // taxon: "",
  // author: site.config.default-author.name,
) = {
  let taxon = attrs.at("taxon", default: none)
  let author = attrs.at("author", default: site.config.default-author.name)
  heading(depth: 1, {
    if taxon != none {
      set text(style: "italic")
      taxon
    }
    context if counter("transclusion-depth").get().at(0) != 0 {
      counter(heading).step(level: counter("transclusion-depth").get().at(0))
    }
    context if counter(heading).get().at(0) != 0 and not _no-numbering {
      if taxon != none { " " } + counter(heading).display() + ". "
    } else if taxon != none { ". " }
    title
  })
  _metadata(date, identifier, ..attrs)
  content
}

#let handle-denote-link(it) = {
  let dest = it.dest
  if type(dest) == str and dest.starts-with("denote:") {
    let identifier = dest.slice(7)
    let new-dest = site.config.base-url + site.config.root-path + "/pdf/" + identifier + ".pdf"
    underline(stroke: (dash: "dotted"), link(new-dest, it.body))
  } else {
    underline(it)
  }
}

#let template(
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

  show link: handle-denote-link

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
    // author: author,
    // taxon: taxon,
    // lang: lang,
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

#let _find-main-content-title-only(c) = {
  let children = c.children
  let p = children.position(it => it.func() == _styled)
  if p != none {
    let child = children.at(p)
    let seq = child.child.children
    let t = seq.position(it => it.func() == heading)
    if t != none {
      _sequence(seq.slice(0, t + 2)) // this is very specific to this template
    } else { none }
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

#let tr(url, hide-metadata: true, open: true, heading-offset: 1) = {
  if url.starts-with("denote:") {
    let identifier = url.slice(7)
    let path = site.id-to-path(identifier)
    let fmc-func = if open {
      _find-main-content
    } else {
      _find-main-content-title-only
    }
    if hide-metadata {
      show label("metadata"): it => none
      transclude(
        path,
        heading-offset: heading-offset,
        transform-heading: x => x,
        transform-other: x => {
          if x.func() == metadata { none } else { x }
        },
        find-main-content: fmc-func,
      )
    } else {
      show label("metadata"): it => it
      transclude(
        path,
        heading-offset: heading-offset,
        transform-heading: x => x,
        transform-other: x => {
          if x.func() == metadata { none } else { x }
        },
        find-main-content: fmc-func,
      )
    }
  }
}

#let backmatters(parts: ()) = {
  set page(
    paper: "us-letter",
    margin: (
      left: 1in,
      right: 1in,
      top: 1.5in,
      bottom: 1.5in,
    ),
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

  for part in parts {
    let (name, urls) = part
    [
      #heading(level: 2, name)
      #for url in urls {
        tr(url, hide-metadata: false, open: false, heading-offset: 2)
      }
    ]
  }
}

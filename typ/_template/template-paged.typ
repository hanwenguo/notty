#import "site.typ"
#import "/_template/lib.typ": plain-text, domain, root-dir, trailing-slash, target, _guard-and-render-metadata, _meta-item-paged as _meta-item

#let id-names-map = json(sys.inputs.at("wb-id-filename-map-file", default: bytes("{}")))

#let sans-fonts = ("Libertinus Sans", "Noto Sans SC", "Source Han Sans SC",)
#let heading-fonts = ("Libertinus Serif Display", "Libertinus Serif",)
#let serif-fonts = ("Libertinus Serif",)

#let _default-metadata(..attrs) = {
  let author = attrs.at("author", default: none)
  let date = attrs.at("date", default: none)
  let metadata-entries = ()
  if date != none { 
    metadata-entries.push(date.display("[month repr:long] [day], [year]"))
  }
  if author != none {
    metadata-entries.push(author.map((a) => { a }).join(", "))
  }
  metadata-entries
}

#let _common-metadata-for-bibliography-entry = (..attrs) => {
  let fields = attrs.at("fields")
  let parsed-names = attrs.at("parsed-names")
  (
    _guard-and-render-metadata("author", (it) => {
      it.map((a) => { 
        let main-name-part = a.given + " " + a.family
        if a.at("prefix", default: "") != "" {
          main-name-part = a.prefix + " " + main-name-part
        }
        if a.at("suffix", default: "") != "" {
          main-name-part = main-name-part + ", " + a.suffix
        }
        main-name-part
      }).join(", ")
    })(parsed-names),
    _guard-and-render-metadata("date", (it) => {
      it
    })(fields),
    _guard-and-render-metadata("doi", (it) => {
      link("https://doi.org/" + it)[#it]
    })(fields),
  )
}

#let metadata-taxon-map-paged = (
  "Person": (..attrs) => {
    (
      _guard-and-render-metadata("position", (it) => {
        it
      })(attrs),
      _guard-and-render-metadata("affiliation", (it) => {
        it
      })(attrs),
      _guard-and-render-metadata("homepage", (it) => {
        link(it)[#it]
      })(attrs),
      _guard-and-render-metadata("orcid", (it) => {
        link("https://orcid.org/" + it)[#it]
      })(attrs),
    )
  },
  "Inproceedings": (..attrs) => {
    let fields = attrs.at("fields")
    (_guard-and-render-metadata("series", (it) => {
      it
    })(fields),) + _common-metadata-for-bibliography-entry(..attrs)
  },
  "Article": (..attrs) => {
    let fields = attrs.at("fields")
    let name = fields.at("shortjournal", default: none)
    if name == none {
      name = fields.at("journal", default: none)
    }
    if fields.at("volume", default: none) != none {
      name = name + " " + fields.at("volume")
    }
    if fields.at("number", default: none) != none {
      name = name + "." + fields.at("number")
    }
    (
      _meta-item(name),
    ) + _common-metadata-for-bibliography-entry(..attrs)
  }
) + site.metadata-taxon-map-paged

#let _metadata(identifier, ..attrs) = {
  let taxon = attrs.at("taxon", default: none)
  let metadata-entries = if taxon != none {
    let f = metadata-taxon-map-paged.at(taxon, default: _default-metadata)
    f(identifier: identifier, ..attrs)
  } else {
    _default-metadata(identifier: identifier, ..attrs)
  }
  block(width: 100%, [
    #set text(font: serif-fonts, size: 11pt)
    #if metadata-entries.len() != 0 { metadata-entries.join([ #sym.dot.c ]) }
    #if metadata-entries.len() != 0 { v(0.5em) }
  ])
}

#let _main-part(
  content,
  title: none,
  identifier: none,
  ..attrs,
) = {
  let taxon = attrs.at("taxon", default: none)
  context heading(
    depth: counter("transclusion-depth").get().at(0) + 1, 
    {
      if taxon != none {
        set text(fill: luma(50%))
        taxon
      }
      context if counter("transclusion-depth").get().at(0) != 0 {
        counter(heading).step(level: counter("transclusion-depth").get().at(0))
      }
      context if counter(heading).get().at(0) != 0 and not state("disable-numbering", false).get() {
        if taxon != none { " " } + counter(heading).display() + ". "
      } else if taxon != none { ". " }
      title
    }
  )
  context if state("show-metadata", true).get() {
    _metadata(identifier, ..attrs)
  }
  context if state("expanded", true).get() {
    content
  }
}

#let wb-section = _main-part

#let template-paged(
  title: "",
  identifier: none,
  ..attrs,
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
      set text(font: serif-fonts, size: 8pt)
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
      set text(hyphenate: false, size: 20pt, font: serif-fonts)
      set par(justify: false, leading: 0.2em, first-line-indent: 0pt)
      title
    }
  }

  show heading.where(depth: 2): it => {
    v(1.3em, weak: true)
    text(size: 14pt, weight: "bold", font: serif-fonts, it)
    v(1em, weak: true)
  }

  show heading.where(depth: 3): it => {
    v(1.3em, weak: true)
    text(size: 13pt, weight: "regular", font: serif-fonts, it)
    v(1em, weak: true)
  }

  show heading.where(depth: 4): it => {
    v(1em, weak: true)
    text(size: 11pt, weight: "light", font: serif-fonts, it)
    v(0.65em, weak: true)
  }

  set par(leading: 0.65em, first-line-indent: 0em, spacing: 1.3em)

  _main-part(
    doc,
    identifier: identifier,
    title: title,
    ..attrs,
  )
}


#let ln-paged(dest, body) = link(dest, body)

#let ct-paged(dest, body) = link(dest, body)

#let tr-paged(url, show-metadata: false, expanded: true, disable-numbering: false, demote-headings: 1) = {
  context state("show-metadata").update(show-metadata)
  context state("expanded").update(expanded)
  context state("disable-numbering").update(disable-numbering)
  context counter("transclusion-depth").update((x) => x + demote-headings)
  let identifier = url.slice(3)
  let path = id-names-map.at(identifier)
  let c = include(path)
  c.children.find((x) => x.func() == [#set text(size: 1pt)].func()).child
  context counter("transclusion-depth").update((x) => x - demote-headings)
  context state("show-metadata").update(false)
  context state("expanded").update(true)
  context state("disable-numbering").update(false)
}

#let inline-tree-paged(
  body,
  identifier: none,
  title: none,
  expanded: true,
  disable-numbering: false,
  ..attrs,
) = {
  context state("expanded").update(expanded)
  context counter("transclusion-depth").step()
  context state("disable-numbering").update(disable-numbering)
  _main-part(
    body,
    identifier: identifier,
    title: title,
    ..attrs,
  )
  context counter("transclusion-depth").update((x) => x - 1)
  context state("expanded").update(true)
  context state("disable-numbering").update(false)
}
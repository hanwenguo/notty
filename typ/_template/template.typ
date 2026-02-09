#import "/_template/site.typ"
#import "/_template/lib.typ": plain-text, domain, root-dir, trailing-slash, target, _guard-and-render-metadata, _meta-item-html as _meta-item
#import "/_template/template-paged.typ": template-paged, ln-paged, ct-paged, tr-paged, inline-tree-paged

#let ln-html(dest, body) = {
  html.span(
    class: "link local",
    html.elem(
      "wb-internal-link",
      attrs: (target: dest),
      body
    )
  )
}

#let ct-html(dest, body) = {
  html.span(
    class: "link local",
    html.elem(
      "wb-cite",
      attrs: (target: dest),
      body
    )
  )
}

#let tr-html(id, show-metadata: false, expanded: true, disable-numbering: false, demote-headings: 1) = {
  html.elem(
    "wb-transclusion",
    attrs: (
      target: id,
      show-metadata: if show-metadata { "true" } else { "false" },
      expanded: if expanded { "true" } else { "false" },
      disable-numbering: if disable-numbering { "true" } else { "false" },
      demote-headings: str(demote-headings),
    )
  )
}

#let _default-metadata = (..attrs) => {
  _guard-and-render-metadata("date", (it) => {
    _meta-item(it.display("[month repr:long] [day], [year]"))
  })(attrs)
  _guard-and-render-metadata("author", (it) => {
    _meta-item(html.address(class: "author", {
      it.map((a) => { a }).join(", ")
    }))
  })(attrs)
  if attrs.at("export-pdf", default: false) {
    _meta-item(link("/pdf/" + attrs.at("identifier", default: "") + ".pdf", "PDF"))
  }
}

#let _common-metadata-for-bibliography-entry = (..attrs) => {
  let fields = attrs.at("fields")
  let parsed-names = attrs.at("parsed-names")
  _guard-and-render-metadata("author", (it) => {
    _meta-item(html.address(class: "author", {
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
    }))
  })(parsed-names)
  _guard-and-render-metadata("date", (it) => {
    _meta-item(it)
  })(fields)
  _guard-and-render-metadata("doi", (it) => {
    _meta-item(html.a(class: "link external", href: "https://doi.org/" + it)[#it])
  })(fields)
}

#let metadata-taxon-map-html = (
  "Person": (..attrs) => {
    _guard-and-render-metadata("position", (it) => {
      _meta-item(it)
    })(attrs)
    _guard-and-render-metadata("affiliation", (it) => {
      _meta-item(it)
    })(attrs)
    _guard-and-render-metadata("homepage", (it) => {
      _meta-item(html.a(class: "link external", href: it)[#it])
    })(attrs)
    _guard-and-render-metadata("orcid", (it) => {
      _meta-item(html.a(
        class: "orcid",
        href: "https://orcid.org/" + it
      )[#it])
    })(attrs)
  },
  "Inproceedings": (..attrs) => {
    let fields = attrs.at("fields")
    _guard-and-render-metadata("series", (it) => {
      _meta-item(it)
    })(fields)
    _common-metadata-for-bibliography-entry(..attrs)
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
    _meta-item(name)
    _common-metadata-for-bibliography-entry(..attrs)
  }
) + site.metadata-taxon-map-html

#let _summary_header(
  level: 1,
  inline: false,
  disable-numbering: false,
  identifier: none,
  title: none,
  ..attrs,
) = {
  let heading-attrs = (:)
  if identifier != none {
    heading-attrs.insert("id", identifier)
  }
  if disable-numbering {
    heading-attrs.insert("class", "disable-numbering")
  }
  html.summary(
    html.header({
      html.elem("h" + str(level), attrs: heading-attrs, {
        if attrs.at("taxon", default: none) != none {
          html.span(class: "taxon", attrs.at("taxon"))
        }
        title
        " "
        if identifier != none {
          let href = if inline {
            "#" + identifier
          } else {
            root-dir + identifier + (if trailing-slash { "/" } else { ".html" })
          }
          html.a(class: "slug", href: href, "[" + identifier + "]")
        }
      })
      html.div(class: "metadata", {
        html.ul(
          metadata-taxon-map-html.at(
            attrs.at("taxon", default: ""),
            default: _default-metadata
          )(identifier: identifier, ..attrs)
        )
      })
    })
  )
}

#let _head(
  identifier: none,
  title: none,
  ..attrs,
) = {
  html.head({
    html.meta(name: "identifier", content: identifier)
    if attrs.at("taxon", default: none) != none {
      html.meta(name: "taxon", content: attrs.at("taxon"))
    }
    if attrs.at("toc", default: true) {
      html.meta(name: "toc", content: "true")
    } else {
      html.meta(name: "toc", content: "false")
    }
    if attrs.at("export-pdf", default: false) {
      html.meta(name: "export-pdf", content: "true")
    } else {
      html.meta(name: "export-pdf", content: "false")
    }
    html.title(plain-text(title))
  })
}

#let _body(
  body,
  identifier: none,
  title: none,
  ..attrs,
) = {
  html.body({
    _summary_header(
      level: 1,
      identifier: identifier,
      title: title,
      ..attrs
    ) 
    body
  })
}

#let inline-tree-html(
  body,
  identifier: none,
  title: none,
  expanded: true,
  disable-numbering: false,
  ..attrs,
) = {
  let details-attrs = if expanded {
    (open: true)
  } else {
    (:)
  }
  html.section(
    class: "block",
    html.details(
      {
        _summary_header(
          level: 2,
          inline: true,
          disable-numbering: disable-numbering,
          identifier: identifier,
          title: title,
          ..attrs
        )
        body
      },
      ..details-attrs,
    )
  )
}

#let template-html(
  identifier: "",
  title: "", 
  ..attrs,
) = (doc) => {
  show math.equation: set text(font: site.config.math-fonts)

  show math.equation.where(block: false): it => {
    {
      set text(site.config.foreground-color.at(0))
      html.span(class: "math-inline color-light", html.frame(it))
    }
    {
      set text(site.config.foreground-color.at(1))
      html.span(class: "math-inline color-dark", html.frame(it))
    }
  }
  show math.equation.where(block: true): it => {
    {
      set text(site.config.foreground-color.at(0))
      html.div(class: "math-display color-light", html.frame(it))
    }
    {
      set text(site.config.foreground-color.at(1))
      html.div(class: "math-display color-dark", html.frame(it))
    }
  }

  // https://github.com/miikanissi/modus-themes.nvim/tree/master/extras/bat
  show raw.where(block: true): it => {
    {
      set raw(theme: "/_template/modus_operandi.tmTheme")
      html.div(class: "color-light", it)
    }
    {
      set raw(theme: "/_template/modus_vivendi.tmTheme")
      html.div(class: "color-dark", it)
    }
  }

  // show raw.where(block: false): it => html.code(it.text)
  // show raw.where(block: true): it => html.pre(it.text)

  show link: it => html.span(
    class: "link external",
    html.a(
      href: it.dest,
      it.body
    )
  )

  show footnote: it => html.aside(it.body)
  
  html.html({
    _head(
      identifier: identifier,
      title: title,
      ..attrs
    )
    _body(
      doc,
      identifier: identifier,
      title: title,
      ..attrs
    )
  })
}

#let template = if target == "html" {
  template-html
} else {
  template-paged
}

#let ln = if target == "html" {
  ln-html
} else {
  ln-paged
}

#let ct = if target == "html" {
  ct-html
} else {
  ct-paged
}

#let tr = if target == "html" {
  tr-html
} else {
  tr-paged
}

#let inline-tree = if target == "html" {
  inline-tree-html
} else {
  inline-tree-paged
}
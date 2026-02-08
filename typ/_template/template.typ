#import "/_template/site.typ"
#import "/_template/template-paged.typ": template-paged, ln-paged, ct-paged, tr-paged, inline-tree-paged

#let domain = sys.inputs.at("wb-domain", default: "")
#let root-dir = sys.inputs.at("wb-root-dir", default: "/")
#let trailing-slash = if sys.inputs.at("wb-trailing-slash", default: "false") == "true" {
  true
} else {
  false
}
#let target = sys.inputs.at("wb-target", default: none)

#let _sequence = [].func()
#let _styled = [#set text(size: 1pt)].func()
#let _equation = $1$.func();

/// Collect text content of element recursively into a single string
/// https://discord.com/channels/1054443721975922748/1088371919725793360/1138586827708702810
/// https://github.com/Myriad-Dreamin/shiroa/issues/55
#let plain-text(it) = {
  if type(it) == str {
    return it
  } else if it == [ ] {
    return " "
  }
  let f = it.func()
  if f == _styled {
    plain-text(it.child)
  } else if f == _equation {
    plain-text(it.body)
  } else if f == text or f == raw {
    it.text
  } else if f == smartquote {
    if it.double {
      "\""
    } else {
      "'"
    }
  } else if f == _sequence {
    it.children.map(plain-text).filter(t => type(t) == str).join()
  } else {
    none
  }
}

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

#let tr-html(id, show-metadata: false, expanded: true, disable-numbering: false, demote-headings: true) = {
  html.elem(
    "wb-transclusion",
    attrs: (
      target: id,
      show-metadata: if show-metadata { "true" } else { "false" },
      expanded: if expanded { "true" } else { "false" },
      disable-numbering: if disable-numbering { "true" } else { "false" },
      demote-headings: if demote-headings { "true" } else { "false" }
    )
  )
}

#let _meta-item(body) = {
  html.li(class: "meta-item", body)
}

#let _guard-and-render-metadata(
  name,
  renderer
) = (attrs) => {
  if attrs.at(name, default: none) != none {
    _meta-item(renderer(attrs.at(name)))
  }
}

#let default-metadata = (..attrs) => {
  _guard-and-render-metadata("date", (it) => {
    it.display("[month repr:long] [day], [year]")
  })(attrs)
  _guard-and-render-metadata("author", (it) => {
    html.address(class: "author", {
      it.map((a) => { a }).join(", ")
    })
  })(attrs)
  if attrs.at("export-pdf", default: false) {
     _meta-item(link("/pdf/" + attrs.at("identifier", default: "") + ".pdf", "PDF"))
  }
}

#let metadata-taxon-map-html = (
  "Person": (..attrs) => {
    _guard-and-render-metadata("position", (it) => {
      it
    })(attrs)
    _guard-and-render-metadata("affiliation", (it) => {
      it
    })(attrs)
    _guard-and-render-metadata("homepage", (it) => {
      html.a(class: "link external", href: it)[#it]
    })(attrs)
    _guard-and-render-metadata("orcid", (it) => {
      html.a(
        class: "orcid",
        href: "https://orcid.org/" + it
      )[#it]
    })(attrs)
  },
)

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
            default: default-metadata
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
#import "/_template/site.typ"
#import "/_template/template-paged.typ": template-paged, ln-paged, ct-paged, tr-paged

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

#let _summary_header(
  level: 1,
  inline: false,
  identifier: none,
  title: none,
  ..attrs,
) = {
  html.summary(
    html.header({
      html.elem("h" + str(level), attrs: (id: identifier), {
        if attrs.at("taxon", default: none) != none {
          html.span(class: "taxon", attrs.at("taxon"))
        }
        title
        " "
        if identifier != none {
          let href = if inline {
            "#" + identifier
          } else {
            site.config.root-path + identifier + (if site.config.trailing-slash { "/" } else { ".html" })
          }
          html.a(class: "slug", href: href, "[" + identifier + "]")
        }
      })
      html.div(class: "metadata", {
        html.ul({
          if attrs.at("date", default: none) != none {
            html.li(class: "meta-item", {
              attrs.at("date").display("[month repr:long] [day], [year]")
            })
          }
        })
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
    if attrs.at("author", default: none) != none {
      html.meta(name: "author", content: attrs.at("author"))
    }
    if attrs.at("date", default: none) != none {
      html.meta(name: "date", content: attrs.at("date").display("[year]-[month]-[day]T[hour]:[minute]:[second]Z"))
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
  // html.body(_section(
  //   body,
  //   level: 1,
  //   identifier: identifier,
  //   title: title,
  //   date: date,
  //   author: author,
  //   ..attrs
  // ))
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

#let inline-tree(
  body,
  identifier: none,
  title: none,
  expanded: true,
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

#let ln = if target == "html" {
  ln-html
} else {
  ln-paged
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

#let ct = if target == "html" {
  ct-html
} else {
  ct-paged
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

#let tr = if target == "html" {
  tr-html
} else {
  tr-paged
}

#import "site.typ"
#import "html-toolkit.typ": *
#import "transclusion-html.typ": transclude

#let _no-numbering = sys.inputs.at("no-numbering", default: none) != none

#let _default-metadata(date, identifier, ..attrs) = {
  let author = attrs.at("author", default: site.config.default-author.name)
  div(
    ul(
      {
        if date != none { li(date.display("[month repr:long] [day], [year repr:full]"), class: "meta-item") }
        let author = attrs.at("author", default: none)
        if author != none { li(author, class: "meta-item") }
        li(
            span(
              a(
                "PDF",
                attrs: (href: "/pdf/" + identifier + ".pdf"),
              ),
              class: "link"
            ),
            class: "meta-item"
          )
      }
    ),
    class: "metadata"
  )
}

#let _metadata(date, identifier, ..attrs) = {
  let taxon = attrs.at("taxon", default: none)
  if taxon != none {
    let f = site.html-metadata-taxon-map.at(taxon, default: _default-metadata)
    f(date, identifier, ..attrs)
  } else {
    _default-metadata(date, identifier, ..attrs)
  }
}

#let _section(
  content,
  title: none,
  date: none,
  identifier: none,
  open: true,
  ..attrs,
  // author: site.config.default-author.name,
  // taxon: none,
  // lang: site.config.lang,
) = {
  let taxon = attrs.at("taxon", default: none)
  let author = attrs.at("author", default: site.config.default-author.name)
  let lang = attrs.at("lang", default: site.config.lang)
  let id = if identifier != none { identifier } else { date.display("[year repr:full][month repr:numerical][day]T[hour repr:24][minute][second]") }
  // let title-prefix = context {
  //   let has-taxon = taxon != none
  //   let show-number = counter(heading).get().at(0) != 0 and _numbering
  //   let has-prefix = has-taxon or show-number
  //   (if has-taxon { taxon })
  //   (if show-number { " " + counter(heading).display() }) + (if has-prefix { ". " })
  //   counter(heading).step(level: counter("transclusion-depth").get().at(0) + 1)
  // }
  section(
    html.elem(
      "details",
      {
        summary(
          header({
            h1(
              {
                span(
                  [#if taxon != none { taxon }#context if counter("transclusion-depth").get().at(0) != 0 { counter(heading).step(level: counter("transclusion-depth").get().at(0)) }#context if counter(heading).get().at(0) != 0 and not _no-numbering { " " + counter(heading).display() + ". " } else if taxon != none { ". " }], 
                  // title-prefix,
                  class: "taxon"
                )
                title
                a(
                  "[" + id + "]",
                  class: "slug",
                  attrs: (
                    href: "/" + id + ".html"
                  )
                )
              },
              // attrs: if taxon != none { (taxon: taxon) } else { (:) }
            )
            _metadata(date, identifier, ..attrs)
          })
        )
        content
      },
      attrs: if open { (open: "") } else { none }
    ),
    class: "block",
    attrs: (lang: lang)
  )
}

#let _main-part(
  content,
  title: none,
  date: none,
  identifier: none,
  ..attrs,
  // taxon: none,
  // author: site.config.default-author.name,
  // lang: site.config.lang,
) = {
  html.elem(
    "html",
    _section(content, title: title, date: date, identifier: identifier, ..attrs)
  )
}

#let template(
  title: "", 
  date: none,
  identifier: none,
  ..attrs,
  // author: none, 
  // taxon: none,
  // lang: site.config.lang,
) = (doc) => {
  set math.equation(numbering: "(1)")
  show raw.where(block: false): it => html.elem("code", it.text)

  show math.equation: set text(fill: color.rgb(235, 235, 235, 90%)) if x-is-dark
  show math.equation: set text(size: 12pt, font: "IBM Plex Math")
  show math.equation.where(block: false): it => html.elem("span", html.frame(it), attrs: ("class": "typst-inline")) 
  show math.equation.where(block: true): div-frame.with(attrs: ("style": "display: flex; justify-content: center; overflow-x: auto;", "class": "typst-display"))
  // show link: handle-denote-link
  show link: (it) => {
    if type(it.dest) == str {
      span(a(it.body, attrs: (href: it.dest)), class: ("link", "external"))
    } else {
      it
    }
  }

  ([#metadata((
    title: title,
    date: date.display("[year repr:full][month repr:numerical][day]T[hour repr:24][minute][second]"),
    identifier: identifier,
  )) <frontmatter>])
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
  set document(title: title, date: date)
}

#let _find-html-root(
  c
) = {
  if c.func() == _sequence {
    let children = c.children
    for child in children {
      let found = _find-html-root(child)
      if found != none {
        return found
      }
    }
  } else if c.func() == _styled {
    let child = _find-html-root(c.child)
    child
  } else if c.func() == html.elem and c.tag == "html" {
    c
  } else {
    none
  }
}

#let _get-html-content-from-root(
  c,
  hide-metadata: true,
  open: true
) = {
  let sec = c.body
  let det = sec.body
  let det-attrs = det.attrs
  if open {
    det-attrs.insert("open", "")
  } else {
    det-attrs.remove("open")
  }
  return html.elem(
    "section",
    html.elem(
      "details",
      det.body,
      attrs: det-attrs
    ),
    attrs: sec.attrs + (if hide-metadata { (class: sec.attrs.at("class", default: "") + " hide-metadata") })
  )
}

#let ln(dest, body) = {
  if type(dest) == str {
    if dest.starts-with("denote:") {
      let identifier = dest.slice(7)
      let new-dest = identifier + ".html"
      span(a(body, attrs: (href: "/" + new-dest)), class: ("link", "local"))
    }
  } else { it }
}

#let tr(url, hide-metadata: true, open: true) = {
  if url.starts-with("denote:") {
    let identifier = url.slice(7)
    let path = site.id-to-path(identifier)
    transclude(
      path,
      heading-offset: 1,
      transform-heading: (x) => x,
      transform-other: (x) => {
        if x.func() == metadata { none } else { x }
      },
      find-html-root: _find-html-root,
      get-html-content-from-root: _get-html-content-from-root.with(hide-metadata: hide-metadata, open: open),
    )
  }
}

#let backmatters(parts: ()) = {
  set math.equation(numbering: "(1)")
  show raw.where(block: false): it => html.elem("code", it.text)

  show math.equation: set text(fill: color.rgb(235, 235, 235, 90%)) if x-is-dark
  show math.equation: set text(size: 12pt, font: "IBM Plex Math")
  show math.equation.where(block: false): it => html.elem("span", html.frame(it), attrs: ("class": "typst-inline")) 
  show math.equation.where(block: true): div-frame.with(attrs: ("style": "display: flex; justify-content: center; overflow-x: auto;", "class": "typst-display"))

  show link: (it) => {
    if type(it.dest) == str {
      span(a(it.body, attrs: (href: it.dest)), class: ("link", "external"))
    } else {
      it
    }
  }

  html.elem(
    "html",
    html.elem(
      "footer",
      for part in parts {
        let (name, urls) = part
        section(
          details(
            {
              summary(
                header({
                  h1({
                    span([], class: "taxon")
                    name
                  })
                })
              )
              for url in urls {
                tr(url, hide-metadata: false, open: false)
              }
            },
            attrs: (open: "")
          ),
          class: "block",
          attrs: (lang: site.config.lang)
        )
      }
    )
  )
}
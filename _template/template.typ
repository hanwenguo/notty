#import "site.typ"
#import "html-toolkit.typ": *
#import "transclusion-html.typ": transclude

#let _main-part(
  content,
  title: none,
  date: none,
  author: none,
  description: none,
) = {
  a(
    href: "/",
    class: "back-link font-monospace",
    if site.config.name != none {
      plain-text(site.config.name)
    }
  )
  html.elem(
    "article",
    {
      h1(if title != none {
        plain-text(title)
      })
      div(
        class: "post-meta",
        {
          html-elem(
            tag: "time",
            if date != none {
              plain-text(date.display())
            }
          )
          if sys.inputs.at("filename", default: none) != none {
            html-elem(
              tag: "a",
              href: "/pdf/" + sys.inputs.at("filename") + ".pdf",
              "PDF"
            )
          }
        }
      )
      content
    }
  )
}

#let handle-denote-link(it) = {
  let dest = it.dest
  if type(dest) == str and dest.starts-with("denote:") {
    let identifier = dest.slice(7)
    let new-dest = identifier + ".html"
    link(new-dest, it.body)
  } else {
    it
  }
}

#let template(
  date: none,
  signature: none,
  title: "", 
  author: none, 
  description: none,
  tags: none,
  identifier: none,
) = (doc) => {
  set math.equation(numbering: "(1)")
  show raw.where(block: false): it => html.elem("code", it.text)

  show math.equation: set text(fill: color.rgb(235, 235, 235, 90%)) if x-is-dark
  show math.equation: div-frame.with(attrs: ("style": "display: flex; justify-content: center; overflow-x: auto;"))

  show heading: (it) => html-elem(tag: "h" + str(it.depth), it.body)
  show link: handle-denote-link

  ([#metadata((
    signature: signature,
    title: title,
    author: author,
    date: date,
  )) <frontmatter>])
  load-html-template(
    "template.html",
    _main-part(
      doc,
      title: title,
      date: date,
      author: author,
      description: description
    ),
    extra-head: {
      if description != none {
        head-meta("description", plain-text(description))
      }
    },
    title: if title != none {
      plain-text(title)
    } else {
      "Untitled"
    }
  )
  set document(title: title, date: date)
}

#let _transcluded-path-to-url(path) = {
  let file-name = path.split("/").at(-1)
  // "20250426T224704"
  // "012345678901234"
  let identifier = file-name.slice(0, 15)
  let new-path = identifier + ".html"
  new-path
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

#let _find-html-content-from-root(
  c
) = {
  let main-part = c.body.children
  .find(it => it.func() == html.elem and it.tag == "body")
  .body.body.children
  .find(it => it.func() == html.elem and it.tag == "article")
  .body
  let children = main-part.children
  let removed = children.remove(
    children.position((it) => {
      if it.func() == html.elem and it.tag == "div" {
        it.attrs.at("class", default: none) == "post-meta"
      } else { false }
    })
  )
  _sequence(children)
}

#let tr(path) = transclude(
  if not path.starts-with("/") {
    "/typ/"
  } else {
    ""
  } + path,
  heading-offset: 1,
  transform-heading: (x) => {
    let b = x.body
    let a = x.attrs
    let d = int(x.tag.at(1))
    if d == 1 and not (b.func() == html.elem and b.body().tag == "a") {
      html.elem(
        x.tag,
        attrs: a,
        html.elem(
          "a",
          attrs: ("href": _transcluded-path-to-url(path)),
          b,
        )
      )
    } else {
      x
    }
  },
  transform-other: (x) => {
    if x.func() == metadata { none } else { x }
  },
  find-html-root: _find-html-root,
  find-html-content-from-root: _find-html-content-from-root,
)
#import "/_template/site.typ"
#import "/_template/lib.typ": (
  _guard-and-render-metadata, _meta-item-html as _meta-item, domain, plain-text, root-dir, trailing-slash,
)

#let all-posts = state("all-posts", ())
#let current-id-stack = state("current-id-stack", ())
#let in-main-content = state("in-main-content", false)

#let _graph-edge(relation, dest) = {
  [#context [#metadata((
    kind: "wb-edge",
    relation: relation,
    dest: dest,
    copy-depth: counter("graph-copy-depth").get().first(),
  )) <wb-edge-record>]]
}

#let _owned-edge-values(id) = {
  let start = label(id + "-main-start")
  let end = label(id + "-main-end")
  query(selector(<wb-edge-record>).after(start).before(end))
    .filter(item => {
      let value = item.value
      if type(value) != dictionary or value.at("kind", default: none) != "wb-edge" {
        return false
      }
      if value.at("copy-depth", default: 0) != 0 {
        return false
      }
      return true
    })
    .map(item => item.value)
}

#let _graph-relations(id, relation) = {
  _owned-edge-values(id)
    .filter(value => value.at("relation", default: none) == relation)
    .map(value => value.at("dest"))
    .dedup()
}

#let _graph-incoming(id, relation) = {
  all-posts.final().filter(x => _graph-relations(x, relation).contains(id))
}

#let _default-metadata = (..attrs) => {
  _guard-and-render-metadata("date", it => {
    _meta-item(it.display("[month repr:long] [day], [year]"))
  })(attrs)
  _guard-and-render-metadata("author", it => {
    _meta-item(html.address(class: "author", {
      it.map(a => { a }).join(", ")
    }))
  })(attrs)
  // if attrs.at("export-pdf", default: false) {
  //   _meta-item(link("/pdf/" + attrs.at("identifier", default: "") + ".pdf", "PDF"))
  // }
}

#let _common-metadata-for-bibliography-entry = (..attrs) => {
  let fields = attrs.at("fields")
  let parsed-names = attrs.at("parsed-names")
  _guard-and-render-metadata("author", it => {
    _meta-item(html.address(class: "author", {
      it
        .map(a => {
          let main-name-part = a.given + " " + a.family
          if a.at("prefix", default: "") != "" {
            main-name-part = a.prefix + " " + main-name-part
          }
          if a.at("suffix", default: "") != "" {
            main-name-part = main-name-part + ", " + a.suffix
          }
          main-name-part
        })
        .join(", ")
    }))
  })(parsed-names)
  _guard-and-render-metadata("date", it => {
    _meta-item(it)
  })(fields)
  _guard-and-render-metadata("doi", it => {
    _meta-item(html.a(class: "link external", href: "https://doi.org/" + it)[#it])
  })(fields)
}

#let metadata-taxon-map-html = (
  (
    "Person": (..attrs) => {
      _guard-and-render-metadata("position", it => {
        _meta-item(it)
      })(attrs)
      _guard-and-render-metadata("affiliation", it => {
        _meta-item(it)
      })(attrs)
      _guard-and-render-metadata("homepage", it => {
        _meta-item(html.a(class: "link external", href: it)[#it])
      })(attrs)
      _guard-and-render-metadata("orcid", it => {
        _meta-item(html.a(
          class: "orcid",
          href: "https://orcid.org/" + it,
        )[#it])
      })(attrs)
    },
    "Inproceedings": (..attrs) => {
      let fields = attrs.at("fields")
      _guard-and-render-metadata("series", it => {
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
    },
  )
    + site.metadata-taxon-map-html
)

#let _calc-toc(post-id, identifier, href, disable-numbering, title-part) = {
  // [`_calc-toc` `post-id`: #post-id]
  let depth = counter("transclusion-depth").get().first()
  // [`_calc-toc` `depth`: #depth]
  let current-stack = state(post-id + "-toc-stack", ())
  let current-result = state(post-id + "-toc-result", ())
  let stack = current-stack.get()
  // [`_calc-toc` `current-stack`: #stack]
  while stack.len() > 0 and stack.last().depth >= depth {
    let top = stack.pop()
    // [`_calc-toc` popped stack `top`: #top]
    // [`_calc-toc` `current-result` #current-result.get()]
    current-result.update(result-stack => {
      let children = ()
      while result-stack.len() > 0 and result-stack.last().depth > top.depth {
        let result-top = result-stack.pop()
        children.push(result-top)
      }
      result-stack.push((
        depth: top.depth,
        identifier: top.identifier,
        href: top.href,
        disable-numbering: top.disable-numbering,
        title-part: top.title-part,
        children: children.rev(),
      ))
      return result-stack
    })
    // [`_calc-toc` new `current-result` #current-result.get()]
  }
  current-stack.update(stack => {
    while stack.len() > 0 and stack.last().depth >= depth {
      stack.pop()
    }
    if depth > 0 {
      stack.push((
        depth: depth,
        identifier: identifier,
        href: href,
        disable-numbering: disable-numbering,
        title-part: title-part,
      ))
    }
    return stack
  })
  // [`_calc-toc` new `current-stack`: #current-stack.get()]
}

#let render-toc(toc-result) = {
  html.ul(
    class: "block",
    {
      for toc-item in toc-result {
        let li-attrs = if toc-item.disable-numbering {
          (class: "disable-numbering")
        } else {
          (:)
        }
        html.li(
          {
            html.a(href: toc-item.href, class: "bullet", "■")
            html.a(
              href: "#" + toc-item.identifier,
              class: "link local",
              toc-item.title-part,
            )
            if toc-item.children.len() > 0 { render-toc(toc-item.children) }
          },
          ..li-attrs,
        )
      }
    },
  )
}

#let toc(id) = {
  context {
    let toc-items = state(id + "-toc-result", ()).get()
    if toc-items.len() > 0 {
      html.nav(
        id: "toc",
        html.div(
          class: "block",
          {
            html.h1("Table of Contents")
            render-toc(toc-items)
          },
        ),
      )
    }
  }
}

#let _summary_header(
  identifier: none,
  title: none,
  inline: false,
  show-metadata: true,
  disable-numbering: false,
  ..attrs,
) = {
  html.summary({
    let heading-attrs = (:)
    if identifier != none {
      heading-attrs.insert("id", identifier)
    }
    html.header({
      html.h1(..heading-attrs, {
        let title-part = {
          if attrs.at("taxon", default: none) != none {
            html.span(class: "taxon", attrs.at("taxon"))
          }
          title
        }
        let href = if inline {
          "#" + identifier
        } else {
          root-dir + identifier + (if trailing-slash { "/" } else { ".html" })
        }
        title-part
        context if in-main-content.get() {
          _calc-toc(current-id-stack.get().first(), identifier, href, disable-numbering, title-part)
        }
        " "
        if identifier != none { html.a(class: "slug", href: href, "[" + identifier + "]") }
      })
      if show-metadata {
        html.div(class: "metadata", {
          html.ul(
            metadata-taxon-map-html.at(
              attrs.at("taxon", default: ""),
              default: _default-metadata,
            )(identifier: identifier, ..attrs),
          )
        })
      }
    })
  })
}

#let html-section(
  identifier: none,
  title: "",
  inline: false,
  show-metadata: true,
  expanded: true,
  disable-numbering: false,
  main-part,
  ..attrs,
) = {
  let details-attrs = if expanded {
    (open: true)
  } else {
    (:)
  }
  html.section(
    class: "block" + if disable-numbering { " disable-numbering" } else { "" },
    lang: attrs.at("lang", default: site.config.default-lang),
    html.details(
      {
        _summary_header(
          identifier: identifier,
          title: title,
          inline: inline,
          show-metadata: show-metadata,
          disable-numbering: disable-numbering,
          ..attrs,
        )
        main-part
      },
      ..details-attrs,
    ),
  )
}

#let html-content-frame(
  identifier: "",
  title: "",
  show-metadata: true,
  expanded: true,
  disable-numbering: false,
  main-part,
  ..attrs,
) = [
  #metadata(attrs) #label(identifier + "-metadata")
  #metadata(title) #label(identifier + "-title")
  #metadata(none) #label(identifier + "-main-start")
  #html-section(
    identifier: identifier,
    title: title,
    show-metadata: show-metadata,
    expanded: expanded,
    disable-numbering: disable-numbering,
    [#html.div(main-part) #label(identifier)],
    ..attrs,
  )
  #metadata(none) #label(identifier + "-main-end")
]

#let tr(id, show-metadata: false, expanded: true, disable-numbering: false) = {
  let details-attrs = if expanded {
    (open: true)
  } else {
    (:)
  }
  let true-id = id.slice(3)
  _graph-edge("transcluded", true-id)
  counter("transclusion-depth").update(x => x + 1)
  counter("graph-copy-depth").update(x => x + 1)
  context if in-main-content.get() { current-id-stack.update(arr => arr + (true-id,)) }
  context html-section(
    identifier: true-id,
    title: query(label(true-id + "-title")).first().value,
    show-metadata: show-metadata,
    expanded: expanded,
    disable-numbering: disable-numbering,
    query(label(true-id)).first(),
    ..query(label(true-id + "-metadata")).first().value,
  )
  context if in-main-content.get() { current-id-stack.update(arr => arr.slice(0, -1)) }

  counter("graph-copy-depth").update(x => x - 1)
  counter("transclusion-depth").update(x => x - 1)
}

#let ln(dest, body) = {
  let true-dest = dest.slice(3)
  _graph-edge("linked", true-dest)
  html.span(
    class: "link local",
    html.a(
      href: root-dir + true-dest + (if trailing-slash { "/" } else { ".html" }),
      body,
    ),
  )
}

#let ct(dest, body) = {
  let true-dest = dest.slice(3)
  _graph-edge("cited", true-dest)
  html.span(
    class: "link local citation",
    html.a(
      href: root-dir + true-dest + (if trailing-slash { "/" } else { ".html" }),
      class: "citation",
      body,
    ),
  )
}

#let inline-tree(
  body,
  identifier: none,
  title: none,
  show-metadata: true,
  expanded: true,
  disable-numbering: false,
  ..attrs,
) = {
  context counter("transclusion-depth").update(x => x + 1)

  let section = html-section(
    identifier: identifier,
    title: if title != none { title } else { "" },
    show-metadata: show-metadata,
    expanded: expanded,
    disable-numbering: disable-numbering,
    html.div(body),
    ..attrs,
  )

  if identifier != none [#section #label(identifier)] else [#section]

  context counter("transclusion-depth").update(x => x - 1)
}

#let backmatter-section(name, ids) = {
  counter("transclusion-depth").update(x => x + 1)
  html-section(
    identifier: none,
    title: name,
    show-metadata: false,
    expanded: true,
    disable-numbering: true,
    html.div(
      (
        for id in ids {
          tr("wb:" + id, show-metadata: true, expanded: false, disable-numbering: true)
        }
      ),
    ),
  )
  counter("transclusion-depth").update(x => x - 1)
}

#let backmatter-contexts(id) = {
  context {
    let contexts = _graph-incoming(id, "transcluded")
    if contexts.len() > 0 {
      backmatter-section(
        "Contexts",
        contexts,
      )
    }
  }
}

#let backmatter-references(id) = {
  context {
    let references = _graph-relations(id, "cited")
    if references.len() > 0 {
      backmatter-section(
        "References",
        references,
      )
    }
  }
}

#let backmatter-backlinks(id) = {
  context {
    let backlinks = _graph-incoming(id, "linked")
    if backlinks.len() > 0 {
      backmatter-section(
        "Backlinks",
        backlinks,
      )
    }
  }
}

#let backmatter-related(id) = {
  context {
    let get-related(x) = {
      let linked = _graph-relations(x, "linked")
      let transcluded = _graph-relations(x, "transcluded")
      let transcluded-linked = transcluded.map(get-related).flatten()
      return (linked + transcluded-linked).filter(x => (not transcluded.contains(x)) and x != id)
    }
    // let related = state(id + "-linked", ()).at(label("after-" + id))
    let related = get-related(id).dedup()
    if related.len() > 0 {
      backmatter-section(
        "Related",
        related,
      )
    }
  }
}

#let html-frame(
  lang: "en",
  identifier: "",
  title: "",
  enable-toc: true,
  main-part,
) = {
  html.html(
    lang: lang,
    {
      html.head({
        html.meta(http-equiv: "content-type", content: "text/html; charset=utf-8")
        html.meta(name: "viewport", content: "width=device-width")
        html.link(rel: "stylesheet", href: "/css/weibian.css")
        html.link(rel: "preconnect", href: "https://fonts.googleapis.com")
        html.link(rel: "preconnect", href: "https://fonts.gstatic.com", crossorigin: "anonymous")
        html.link(
          rel: "stylesheet",
          href: "https://fonts.googleapis.com/css2?family=Source+Sans+3:ital,wght@0,200..900;1,200..900&family=Source+Serif+4:ital,opsz,wght@0,8..60,200..900;1,8..60,200..900&display=swap",
        )
        html.link(rel: "stylesheet", href: "/css/stix-two-math.css")
        html.meta(name: "identifier", content: identifier)
        html.meta(name: "lang", content: lang)
        html.title(plain-text(title))
      })
      html.body(
        html.div(
          id: "grid-wrapper",
          {
            html.header(
              class: "header",
              if identifier != "index" {
                html.nav(
                  class: "nav",
                  {
                    html.div(
                      class: "logo",
                      {
                        html.a(href: root-dir, title: "Home")[« Home]
                      },
                    )
                  },
                )
              } else { none },
            )
            html.article({
              counter("transclusion-depth").update(0)
              counter("graph-copy-depth").update(0)

              let this-toc-stack = state(identifier + "-toc-stack", ())
              let this-toc-result = state(identifier + "-toc-result", ())

              all-posts.update(arr => if arr.contains(identifier) { arr } else { arr + (identifier,) })
              current-id-stack.update((identifier,))
              in-main-content.update(true)
              main-part
              context _calc-toc(identifier, none, none, none, none)
              [#metadata("placeholder") #label("after-" + identifier)]
              in-main-content.update(false)
              current-id-stack.update(())

              if identifier != "index" {
                html.footer({
                  backmatter-contexts(identifier)
                  backmatter-references(identifier)
                  backmatter-backlinks(identifier)
                  backmatter-related(identifier)
                })
              }
            })
            if enable-toc { toc(identifier) }
          },
        ),
      )
    },
  )
}

#let template(
  identifier: "",
  title: "",
  ..attrs,
) = doc => {
  let _path = if identifier == "index" {
    "index.html"
  } else {
    if trailing-slash {
      identifier + "/index.html"
    } else {
      identifier + ".html"
    }
  }
  document(_path, title: title, {
    show link: it => html.span(
      class: "link external",
      html.a(
        href: it.dest,
        it.body,
      ),
    )

    show footnote: it => html.aside(it.body)
    html-frame(
      lang: attrs.at("lang", default: site.config.default-lang),
      enable-toc: attrs.at("toc", default: true),
      identifier: identifier,
      title: title,
      html-content-frame(identifier: identifier, title: title, doc, ..attrs),
    )
  })
}

#import "/_template/site.typ": config
#import "/_template/template-paged.typ": template-paged, ln-paged, tr-paged

#let target = sys.inputs.at("notty-target", default: none)

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
  title: none,
  identifier: none,
  inline: false,
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
            config.root-path + identifier + ".html"
          }
          html.a(class: "slug", href: href, "[" + identifier + "]")
        }
      })
      html.div(class: "metadata", {
        html.ul({
        })
      })
    })
  )
}

// #let _section(
//   body,
//   level: 1,
//   identifier: none,
//   title: none,
//   ..attrs,
// ) = {
//   html.section(
//     class: "block",
//     html.details(
//       open: true,
//       {
//         html.summary(html.header({
//           html.elem("h" + str(level), attrs: (id: identifier), {
//             if attrs.at("taxon", default: none) != none {
//               html.span(class: "taxon", attrs.at("taxon"))
//             }
//             title
//             " "
//             if identifier != none {
//               html.a(class: "slug", href: config.root-path + identifier + ".html", "[" + identifier + "]")
//             }
//           })
//           html.div(class: "metadata", {
//             html.ul({
//             })
//           })
//         }))
//         body
//       }
//     )
//   )
// }

#let _head(
  identifier: none,
  title: none,
  date: none,
  author: none,
  ..attrs,
) = {
  html.head({
    html.meta(name: "identifier", content: identifier)
    html.meta(name: "author", content: author)
    html.meta(name: "date", content: date.display("[year]-[month]-[day]T[hour]:[minute]:[second]Z"))
    html.title(plain-text(title))
  })
}

#let _body(
  body,
  identifier: none,
  title: none,
  date: none,
  author: none,
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
      identifier: identifier,
      title: title,
      level: 1,
      ..attrs
    ) 
    body
  })
}

#let inline-tree(
  body,
  identifier: none,
  title: none,
  ..attrs,
) = html.section(
    class: "block",
    html.details(
      open: true,
      {
        _summary_header(
          identifier: identifier,
          title: title,
          level: 2,
          inline: true,
          ..attrs
        )
        body
      }
    )
  )

#let template-html(
  title: "", 
  date: datetime.today(),
  author: "",
  identifier: "",
  ..attrs,
  // taxon: none,
  // lang: site.config.lang,
) = (doc) => {
  show math.equation.where(block: false): it => html.span(class: "math-inline", html.frame(it))
  show math.equation.where(block: true): it => html.div(class: "math-display", html.frame(it))

  show raw.where(block: false): it => html.code(it.text)
  show raw.where(block: true): it => html.pre(it.text)

  show footnote: it => html.aside(it.body)
  
  html.html({
    _head(
      title: title,
      date: date,
      author: author,
      identifier: identifier,
      ..attrs
    )
    _body(
      doc,
      title: title,
      date: date,
      author: author,
      identifier: identifier,
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
      "notty-internal-link",
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

#let tr-html(id, show-metadata: false, expanded: true, disable-numbering: false, demote-headings: true) = {
  html.elem(
    "notty-transclusion",
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

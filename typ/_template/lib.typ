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

#let domain = sys.inputs.at("wb-domain", default: "")
#let root-dir = sys.inputs.at("wb-root-dir", default: "/")
#let trailing-slash = if sys.inputs.at("wb-trailing-slash", default: "false") == "true" {
  true
} else {
  false
}
#let target = sys.inputs.at("wb-target", default: none)

#let _meta-item-html(body) = {
  html.li(class: "meta-item", body)
}

#let _meta-item-paged(body) = {
  body
}

#let _guard-and-render-metadata(
  name,
  renderer
) = (attrs) => {
  if attrs.at(name, default: none) != none {
    renderer(attrs.at(name))
  }
}
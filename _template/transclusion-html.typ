#let _sequence = [].func()
#let _styled = [#set text(size: 1pt)].func()
#let _equation = $1$.func();

#let _offset-one-heading(it, offset: 0) = {
  let tag = it.tag
  let attrs = it.attrs
  let body = it.body
  let heading-number = tag.at(1)
  let new-heading-number = str((int(heading-number) + offset))
  let new-tag = "h" + new-heading-number
  html.elem(
    new-tag,
    attrs: attrs,
    body
  )
}

#let _offset-headings-in-content(
  c,
  offset: 0,
  transform-heading: (x) => x,
  transform-other: (x) => x,
) = {
  if c != none {
    if c.func() == _sequence {
      let children = c.children
      _sequence(children.map(child => _offset-headings-in-content(child, offset: offset, transform-heading: transform-heading, transform-other: transform-other)))
    } else if c.func() == _styled {
      let child = _offset-headings-in-content(c.child, offset: offset, transform-heading: transform-heading, transform-other: transform-other)
      _styled(child, c.styles)
    } else if c.func() == html.elem {
      let tag = c.tag
      if tag == "section" {
        let body = _offset-headings-in-content(c.body, offset: offset, transform-heading: transform-heading, transform-other: transform-other)
        let attrs = c.attrs
        html.elem(
          "section",
          attrs: attrs,
          body
        )
      } else if tag.starts-with("h") and tag.len() == 2 and "123456".contains(tag.at(1)) {
        transform-heading(_offset-one-heading(c, offset: offset))
      } else {
        let body = _offset-headings-in-content(c.body, offset: offset, transform-heading: transform-heading, transform-other: transform-other)
        let attrs = c.attrs
        transform-other(html.elem(
          tag,
          attrs: attrs,
          body
        ))
      }
    } else {
      transform-other(c)
    }
  }
}

#let transclude(
  path,
  heading-offset: 0,
  transform-heading: (x) => x,
  transform-other: (x) => x,
  find-html-root: (x) => x,
  get-html-content-from-root: (x) => x,
) = {
  let doc = include(path);
  let html-root = find-html-root(doc)
  let main-part = get-html-content-from-root(html-root)
  doc = _offset-headings-in-content(main-part, offset: heading-offset, transform-heading: transform-heading, transform-other: transform-other)
  {
    counter("transclusion-depth").step()
    doc
    counter("transclusion-depth").update((x) => x - 1)
  }
}
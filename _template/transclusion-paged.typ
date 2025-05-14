#let _sequence = [].func()
#let _styled = [#set text(size: 1pt)].func()
#let _equation = $1$.func();

#let _offset-one-heading(it, offset: 0) = {
  if it.func() == heading {
    let fields = it.fields()
    let _level = auto
    let _depth = 1
    let _offset = 0
    let other-attrs = (:)
    // let _numbering = none
    // let _supplement = auto
    // let _outlined = true
    // let _bookmarked = auto
    // let _hanging-indent = auto
    for (k, v) in fields {
      if (k == "level") {
        _level = v
      } else if (k == "depth") {
        _depth = v
      } else if (k == "offset") {
        _offset = v
      }
      else if (k == "numbering") {
        other-attrs.insert("numbering", v)
        // _numbering = v
      } else if (k == "supplement") {
        other-attrs.insert("supplement", v)
        // _supplement = v
      } else if (k == "outlined") {
        other-attrs.insert("outlined", v)
        // _outlined = v
      } else if (k == "bookmarked") {
        other-attrs.insert("bookmarked", v)
        // _bookmarked = v
      } else if (k == "hanging-indent") {
        other-attrs.insert("hanging-indent", v)
        // _hanging-indent = v
      }
    }
    if (_level != auto) {
      _depth = _level + offset
    } else {
      _depth = _depth + _offset + offset
    }
    heading(
      depth: _depth,
      offset: 0,
      ..other-attrs,
      it.body
    )
  } else {
    it
  }
}

#let _offset-headings-in-content(
  c,
  offset: 0,
  transform-heading: (x) => x,
  transform-other: (x) => x,
) = {
  if c.func() == _sequence {
    let children = c.children
    _sequence(children.map(child => _offset-headings-in-content(child, offset: offset, transform-heading: transform-heading, transform-other: transform-other)))
  } else if c.func() == _styled {
    let child = _offset-headings-in-content(c.child, offset: offset, transform-heading: transform-heading, transform-other: transform-other)
    _styled(child, c.styles)
  } else if c.func() == heading {
    transform-heading(_offset-one-heading(c, offset: offset))
  } else {
    transform-other(c)
  }
}

#let transclude(
  path,
  heading-offset: 0,
  transform-heading: (x) => x,
  transform-other: (x) => x,
  find-main-content: none
) = {
  let doc = include(path);
  let main-part = find-main-content(doc)
  doc = _offset-headings-in-content(main-part, offset: heading-offset, transform-heading: transform-heading, transform-other: transform-other)
  {
    counter("transclusion-depth").step()
    doc
    counter("transclusion-depth").update((x) => x - 1)
  }
}
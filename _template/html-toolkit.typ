#import "transclusion-paged.typ": _styled, _equation, _sequence

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

/// = HTML Toolkit
///
/// This package provides a set of utility functions for working with HTML export.

/// Creates a HTML element.
///
/// - content (content): The content of the element.
/// - tag (str): The tag of the element.
#let html-elem(content, tag: "div", class: (), attrs: (:)) = html.elem(
  tag,
  content,
  attrs: (attrs + 
    if type(class) == array { 
      if class.len() != 0 { (class: class.join(" ")) }
    } else if type(class) == str {
      (class: class)
    }
  ),
)

/// Creates a ```html <a>``` element with the given content.
#let a = html-elem.with(tag: "a")
/// Creates a ```html <span>``` element with the given content.
#let span = html-elem.with(tag: "span")
/// Creates a ```html <div>``` element with the given content.
#let div = html-elem.with(tag: "div")
/// Creates a ```html <style>``` element with the given content.
#let html-p = html-elem.with(tag: "p")
#let style = html-elem.with(tag: "style")
#let hn(n) = html-elem.with(tag: "h" + str(n))
/// Creates a ```html <h1>``` element with the given content.
#let h1 = hn(1)
/// Creates a ```html <h2>``` element with the given content.
#let h2 = hn(2)

#let section = html-elem.with(tag: "section")
#let details = html-elem.with(tag: "details")
#let summary = html-elem.with(tag: "summary")
#let header = html-elem.with(tag: "header")

#let ol = html-elem.with(tag: "ol")
#let ul = html-elem.with(tag: "ul")
#let li = html-elem.with(tag: "li")

/// Creates an embeded block typst frame.
#let div-frame(content, attrs: (:)) = html.elem("div", html.frame(content), attrs: attrs)

// #let on-html(f) = (it) => context if target() == "html" { f(it) } else { it }
// #let on-html-or-else(f, g) = (it) => context if target() == "html" { f(it) } else { g(it) }

/// The target for the HTML export.
///
/// Avaiable targets:
/// - `web-light`: Light theme for web.
/// - `web-dark`: Dark theme for web.
/// - `pdf`: PDF export.
#let x-target = sys.inputs.at("x-target", default: "web-light")
/// Whether the target uses a dark theme.
#let x-is-dark = x-target.ends-with("dark")
/// Whether the target uses a light theme.
#let x-is-light = x-target.ends-with("light")

/// CLI sets the `x-url-base` to the base URL for assets. This is needed if you host the website on the github pages.
///
/// For example, if you host the website on `https://username.github.io/project/`, you should set `x-url-base` to `/project/`.
#let assets-url-base = sys.inputs.at("x-url-base", default: none)
/// The base URL for content.
#let url-base = if assets-url-base != none { assets-url-base } else { "/dist/" }
/// The base URL for assets.
#let assets-url-base = if assets-url-base != none { assets-url-base } else { "/" }

/// Converts the path to the asset to the URL.
///
/// - path (str): The path to the asset.
/// -> str
#let asset-url(path) = {
  if path != none and path.starts-with("/") {
    assets-url-base + path.slice(1)
  } else {
    path
  }
}
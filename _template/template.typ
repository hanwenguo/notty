#import "/_template/template-html.typ": template as html-template, tr as html-tr, ln as html-ln, backmatters as html-backmatters
#import "/_template/template-paged.typ": template as paged-template, tr as paged-tr, ln as paged-ln, backmatters as paged-backmatters

#let target = sys.inputs.at("x-target", default: none)

#let template = if target == "html" { html-template } else { paged-template }
#let tr = if target == "html" { html-tr } else { paged-tr }
#let ln = if target == "html" { html-ln } else { paged-ln }
#let backmatters = if target == "html" { html-backmatters } else { paged-backmatters }
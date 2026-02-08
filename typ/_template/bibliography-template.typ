#import "@preview/citegeist:0.2.1": load-bibliography

#let bib = load-bibliography(read("/references.bib"))

#let template(
  identifier: "",
  title: "",
  ..attrs
) = {
  let bib-entry = bib.at(identifier)
  let title = bib-entry.fields.title
  let taxon = upper(bib-entry.entry_type.first()) + bib-entry.entry_type.slice(1)
  let fields = bib-entry.fields + attrs.named().at("fields", default: (:))

  import "/_template/template.typ": ln, template, tr, inline-tree
  show: template(
    title: title,
    taxon: taxon,
    identifier: identifier,
    toc: false,
    fields: fields,
    parsed-names: bib-entry.parsed_names,
  )

  if fields.at("abstract", default: none) != none [*Abstract*: #fields.at("abstract")]
}
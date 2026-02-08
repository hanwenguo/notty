#import "@preview/citegeist:0.2.1": load-bibliography

#let bib = load-bibliography(read("/" + sys.inputs.at("wb-bib-file").slice(4)))
#let citation-key = sys.inputs.at("wb-bib-key")
#let bib-entry = bib.at(citation-key)
#let title = bib-entry.fields.title
#let taxon = upper(bib-entry.entry_type.first()) + bib-entry.entry_type.slice(1)
#let identifier = bib-entry.entry_key
#let fields = bib-entry.fields

#import "/_template/template.typ": ln, template, tr, inline-tree
#show: template(
  title: title,
  taxon: taxon,
  identifier: identifier,
  toc: false,
  fields: fields,
  parsed-names: bib-entry.parsed_names,
)

#if fields.at("abstract", default: none) != none {
  fields.at("abstract")
}
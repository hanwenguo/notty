#import "/_template/template.typ": template, tr, ln
#show: template(
  title:      [Taking scientific notes need modular reusable snippets for mathematics],
  date:       datetime(year: 2025, month: 08, day: 19, hour: 22, minute: 07, second: 49),
  tags:       (),
  author: (ln("wb:hanwenguo")[Hanwen Guo],),
  identifier: "0002",
  export-pdf: true,
)

Scientific notes usually contains mathematical contents that are typeset in some language, most commonly LaTeX. As the note base grows, shared snippets would appear in clusters of notes. These snippets, if not made reusable as macros or functions in the typesetting language, will become unmaintainable just like how a program without usage of variables and functions are unmaintainable. In the meantime, making all the snippets accessible globally risks leakage of abstraction and namespace pollution. Hence, these typesetting snippets should act just like definitions in programming languages: reusable by referring to their name, and scoped under a module system.

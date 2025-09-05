#import "/_template/template.typ": template, tr, ln
#show: template(
  title:      [Markdown is not suitable for scientific content producing],
  date:       datetime(year: 2025, month: 08, day: 19, hour: 22, minute: 08, second: 12),
  tags:       (),
  identifier: "20250819T220812",
)

Given that #ln("notty:20250819T220749")[Taking scientific notes need modular reusable snippets for mathematics] and #ln("notty:20250819T220803")[Taking scientific notes needs full-power mathematical typesetting], it is obvious that Markdown, without extensions, does not satisfy any of the two principles, and any of the software-specific extensions that tries to solve these problems does not really solve them, just causing migration friction. This is because mathematics is alien to Markdown --- it is just not part of it, thus there cannot be effective integration.

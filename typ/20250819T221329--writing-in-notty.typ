#import "/_template/template.typ": template, tr
#show: template(
  title:      [Writing in Notty],
  date:       datetime(year: 2025, month: 08, day: 19, hour: 22, minute: 13, second: 29),
  tags:       (),
  identifier: "20250819T221329",
)

Every note should start with the following template:

```typst
#import "/_template/template.typ": template, tr, ln
#show: template(
  title:      [Title],
  date:       datetime(year: 2025, month: 08, day: 19, hour: 22, minute: 13, second: 29),
  tags:       (),
  identifier: "20250819T221329",
)

# the content
```

Technically, the `tags` field is optional, and the `identifier` field can be arbitrary instead of a date string. To link to other notes, use `#ln("notty:id")[text]`. To transclude other notes, use `#tr("notty:id", hide-metadata: true, open: true)`; the last two parameters are optional and values here are default values. `hide-metadata` means does not display metadata like date and author under the title, and `open` means show the content for the transcluded note when outputting to HTML. Currently, it is recommended to create hierarchy of notes only through transclusion.

#tr("notty:20250819T221344", open: false)

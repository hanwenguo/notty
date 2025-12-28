#import "/_template/template.typ": template, tr, inline-tree
#show: template(
  title:      [Writing in Notty],
  date:       datetime(year: 2025, month: 08, day: 19, hour: 22, minute: 13, second: 29),
  tags:       (),
  identifier: "20250819T221329",
)

Notty works by exporting your notes written in Typst format to HTML using the HTML export feature of Typst, then post-processing the exported HTML files to create a website. Therefore, writing notes in Notty is essentially writing Typst documents with some conventions. For a demonstration of these conventions, see the #link("https://github.com/hanwenguo/notty")[repository of Notty] itself.

More specifically, Notty will first export each note in the input directory to an HTML file. This does not involve any special processing; the Typst files are compiled as-is, and user is responsible for generating HTML files with the Notty conventions described below. The generated HTML files is not used for display directly, but rather as an intermediate representation for further processing.

The `<head>` section of the generated HTML files may include metadata `<meta>` tags as well as other standard tags such as `<title>`, `<link>`, `<style>`, or `<script>`. Out of the `<meta>` tags, Notty recognizes a special one: `<meta name="identifier" content="...">`: specifies the unique identifier of the note. This is required for every note.

In the HTML files, there could be two special custom elements: `<notty-transclusion target="notty:..." show-metadata="..." expanded="..."></notty-transclusion>` and `<notty-internal-link target="notty:...">...</notty-internal-link>`. The former is used to represent transcluded notes, while the latter is used for internal links between notes. For `<notty-transclusion>`, its body must be empty; the `target` attribute, starting with `notty:`, specifies the ID (not including the `notty:` prefix) of the note to be transcluded, while `show-metadata` and `expanded` are boolean attributes that control the display of metadata and whether the transclusion is expanded by default, respectively; due to limitations in Typst's HTML export capabilities, their values are represented as strings ("true" or "false"). For `<notty-internal-link>`, the `target` attribute specifies the ID of the note to link to, and its body contains the link text.

Notty then post-processes the intermediate HTML files to produce the final HTML files with all transclusions and internal links resolved and backmatter generated. The generated HTML files are placed in an output directory, with each note's HTML file named after its ID (e.g., a note with ID `note-123` is saved as `note-123.html`). The directory structure of these notes should be flat, i.e., all notes are placed directly under the output directory without any subdirectories.

The detailed process is as follows:

1. Parse the HTML files to build a transclusion graph with respect to the `<notty-transclusion>` elements.
2. Process the notes in topological order. For each note, the aforementioned custom elements are replaced with the actual content they represent. For `<notty-internal-link>`, it is replaced with an `<a>` element that links to the target note's HTML file. For `<notty-transclusion>`, it is replaced by the content of the `<body>` of the target note's final HTML file. Since the notes are processed in topological order, the target note should have already been processed when processing the current note. If `show-metadata` is false, the outermost tag of the transcluded content is appended with class `hide-metadata`. If `expanded` is false, the outermost `<details>` tag (might be nested) of the transcluded content will have the `open` attribute removed. After this step, there should be no remaining custom elements in any note.
3. After all transclusions and internal links have been resolved, backlinks and contexts are generated for each note. A backlink from note A to note B exists if note B links to note A via an internal link. A context for note A is defined as any note that directly transcludes note A. The contents of the backlinks and contexts (and generally, any backmatter section) are created by transcluding a virtual note that in turn transcludes all relevant notes. These virtual notes do not exist as actual files, but are constructed on-the-fly during backend processing, and this wouldn't affect the transclusion graph. After this step, a set of backmatter sections is obtained for each note.
4. Finally, the final HTML file for each note will be constructed. There will be a template HTML file that defines the overall structure of the final HTML files, placed in `_template/template.html`. The content of each note replaces the `<slot name="content"></slot>` of the template, while the generated backmatter sections replace the `<slot name="backmatters"></slot>`. (The use of `<slot>` here is just a placeholder because of the name of the tag; no actual Web Components functionality is involved.) Also, any tags in the `<head>` section of the note's HTML file will be appended to the `<head>` section of the template. Template conditionals can be expressed with `<template id="...">` blocks: if the intermediate HTML contains a `<meta name="hide:ID">` tag, the corresponding template block is omitted; otherwise the `<template>` tag is replaced by its content. The resulting HTML file will be saved as the final output for the note.

After the above processing, the output directory will contain the final HTML files for all notes, with all transclusions and internal links resolved, and backmatter generated.

#inline-tree(
  id: "using-configuration-file",
  title: "Using Configuration File"
)[
Notty supports a `.notty/config.toml` configuration file to allow users to set project options such as input/output directories, public assets directory, and other preferences. CLI flags override config values.

The following is an example  configuration file:

```toml
[directories]
input_dir = "typ"
output_dir = "dist"
public_dir = "public"
cache_dir = ".notty/cache"
# the above is the equivalent of the corresponding CLI flags

[site]
domain = "example.com" # the domain of the site; used for generating absolute URLs
root_dir = "/" # the root directory of the site; for example, if the site is hosted at example.com/notes/, set root_dir = "/notes/"
trailing_slash = true # if true, the final URL of each note will have a trailing slash
```

The configuration file is parsed at the start of the program, and values are used as defaults for the corresponding CLI flags. By default, the configuration file is looked for in `.notty/config.toml` relative to the project root, but a different path can be specified with `--config-file <PATH>`. Site settings can also be overridden via CLI with `--site-domain`, `--site-root-dir`, and `--trailing-slash <BOOL>`.

The `trailing_slash` option will affect how internal links are generated and how the output files are organized. If `trailing_slash` is true, each note will be saved in a subdirectory named after its ID, with an `index.html` file inside (e.g., a note with ID `note-123` will be saved as `dist/note-123/index.html`). If false, each note will be saved directly as an HTML file named after its ID (e.g., `dist/note-123.html`). The `root_dir` setting only affects link generation; it does not change where files are written. Special case: a note with ID `index` is always saved as `dist/index.html` and links to the site root.
]

#tr("notty:20250819T221344", expanded: false)

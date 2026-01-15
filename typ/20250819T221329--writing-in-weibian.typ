#import "/_template/template.typ": template, tr, ln, inline-tree
#show: template(
  title:      [Writing in Weibian],
  date:       datetime(year: 2025, month: 08, day: 19, hour: 22, minute: 13, second: 29),
  tags:       (),
  author: ("hanwenguo",),
  identifier: "20250819T221329",
)

Weibian works by exporting your notes written in Typst format to HTML using the HTML export feature of Typst, then post-processing the exported HTML files to create a website. Therefore, writing notes in Weibian is essentially writing Typst documents with some conventions. For a demonstration of these conventions, see the #link("https://github.com/hanwenguo/weibian")[repository of Weibian] itself.

#inline-tree(
  identifier: "rendering-process",
  title: "Rendering Process"
)[

More specifically, Weibian will first export each note in the input directory to an HTML file. This does not involve any special processing; the Typst files are compiled as-is, and user is responsible for generating HTML files with the Weibian conventions described below. The generated HTML files is not used for display directly, but rather as an intermediate representation for further processing.

In the `<body>` of the HTML file, there could be three special custom elements: `<wb-transclusion target="wb:..." show-metadata="..." expanded="..." disable-numbering="..." demote-headings="..."></wb-transclusion>`, `<wb-internal-link target="wb:...">...</wb-internal-link>` and `<wb-cite target="wb:..."></wb-cite>`. 
`<wb-transclusion>` is used to represent transcluded notes, `<wb-internal-link>` is used for internal links between notes, and `<wb-cite>` is used for citations to notes, which is basically a special kind of internal link.
For `<wb-transclusion>`, its body must be empty; the `target` attribute, starting with `wb:`, specifies the ID (not including the `wb:` prefix) of the note to be transcluded, while `show-metadata` and `expanded` are boolean attributes that control the display of metadata and whether the transclusion is expanded by default, respectively; due to limitations in Typst's HTML export capabilities, their values are represented as strings ("true" or "false"). For `<wb-internal-link>`, the `target` attribute specifies the ID of the note to link to, and its body contains the link text. For `<wb-cite>`, the `target` attribute specifies the ID of the note to cite, and its body contains the citation text.

Then, Weibian extracts information from the generated HTML files, and use the Tera templating engine and user-supplied templates to produce the final HTML files for the notes. By default, Weibian looks for templates in `.wb/templates/`. 

This rendering process begins by parsing the HTML files to build a transclusion graph with respect to the `<wb-transclusion>` elements. Then, the notes are processed in topological order. For each note, the aforementioned custom elements are replaced with the actual content they represent.

First, the transclusion and linking relationships are analyzed to build a transclusion graph. Each note is represented as a node in the graph, and a directed edge from node A to node B exists if note A transcludes note B. If there are cycles in the transclusion graph, Weibian will report an error and abort the rendering process, as cyclic transclusions are not supported.

Then, transclusions are processed. For `<wb-transclusion>`, it is rendered via the `transclusion.html` template, which is provided with a a `transclusion` context (`transclusion.target`, `transclusion.show_metadata`, `transclusion.expanded`, `transclusion.hide_numbering`, `transclusion.demote_headings`, `transclusion.content`). The `transclusion.target`, `transclusion.show_metadata`, `transclusion.expanded`, `transclusion.hide_numbering`, and `transclusion.demote_headings` are extracted from the corresponding attributes of the `<wb-transclusion>` element, while `transclusion.content` is the processed content of the target note's final HTML file, to help simplify transclusion rendering in templates. Two Tera filters are registered to help transclusion rendering: `wb_hide_numbering` and `wb_demote_headings`. They apply unconditionally; template conditionals decide whether to invoke them (see the default `transclusion.html`). The result of rendering this template replaces the corresponding `<wb-transclusion>` element in the final HTML file. By processing the notes in topological order, the target note should have already been processed when processing the current note. After this step, there should be only `<wb-internal-link>` and `<wb-cite>` elements left in the HTML file.

For `<wb-internal-link>`, it is rendered via the `internal_link.html` template, which is provided with a `link` context (`link.target`, `link.text`, `link.href`). The `link.target` and `link.text` are extracted from the corresponding attributes and body of the `<wb-internal-link>` element, while `link.href` is the generated URL to the target note's final HTML file, to help simplify link generation in templates. The result of rendering this template replaces the corresponding `<wb-internal-link>` element in the final HTML file. The rendering process for `<wb-cite>` is similar, except that it uses the `citation.html` template and a `citation` context (`citation.target`, `citation.text`, `citation.href`).

Then, backmatters are generated for each note. As for now, Weibian supports four types of backmatter sections: contexts, references, backlinks, and related notes.
- A context for note A is defined as any note that directly transcludes note A.
- A reference from note A to note B exists if note A links to note B via an citation link.
- A backlink from note A to note B exists if note B links to note A via an internal link.
- A related note to note A to note B exists if note A links to note B via an internal link.
The content of each backmatter section is produced by transcluding a virtual note that in turn transcludes all notes relevant to that backmatter section with options `show-metadata="true"`, `expanded="false"`, `disable-numbering="true"`, and `demote-headings="true"`; these virtual notes do not exist as actual files, but are constructed on-the-fly during backend processing, and this wouldn't affect the transclusion graph. Then, each backmatter section is rendered via the `backmatter_section.html` template, which is provided with a `backmatter_section` context (`backmatter_section.title`, `backmatter_section.content`). The `backmatter_section.title` is the title of the backmatter section (e.g., "Backlinks", "Contexts"), while `backmatter_section.content` is the raw HTML of the body of the virtual note described above.

Finally, the final HTML file for each note will be constructed. The template for that is `note.html`. It receives a `note` context (`note.id`, `note.title`, `note.metadata`, `note.head`, `note.content`, `note.toc`, `note.backmatter`). `note.id` and `note.title` are extracted from the corresponding `<meta>` tags, provided for convenience.  The `note.metadata` is a map of metadata key-value pairs extracted from `<meta>` tags with `name` and `content` attributes in the `<head>` section of the intermediate HTML file. The `note.head` is the raw HTML content of the `<head>` section of the intermediate HTML file. `note.content` is the processed content of the `<body>` section of the intermediate HTML file, with all transclusions and internal links resolved as described above. `note.backmatter` is the raw HTML content of the backmatter sections generated also as described above. The `toc` field is an array of `Heading` objects, where each `Heading` object has the following structure:

```
// The hX level
level: 1 | 2 | 3 | 4 | 5 | 6;
// The `id` attribute of the heading tag
id: String;
// The inner HTML of the heading tag
content: String;
// Whether the heading has the "disable-numbering" class
disable_numbering: Bool;
// All lower level headers below this header
children: Array<Heading>;
```

Along all the rendering process, a `site` context (`site.root_dir`, `site.trailing_slash`, `site.domain`) is also provided to all templates to help with link generation and other site-wide settings.
]

#inline-tree(
  identifier: "using-configuration-file",
  title: "Using Configuration File"
)[
Weibian supports a `.weibian/config.toml` configuration file to allow users to set project options such as input/output directories, public assets directory, and other preferences. CLI flags override config values.

The following is an example configuration file:

```toml
[files]
input_dir = "typ"
output_dir = "dist"
public_dir = "public"
# cache_dir = ".wb/cache" # optional; defaults to a project-specific temp dir if omitted
# include = ["**/*.typ"]  # optional; defaults to all files in input_dir
# exclude = ["draft-*"]   # optional; exclude has priority over include
# the above is the equivalent of the corresponding CLI flags

[site]
domain = "example.com" # the domain of the site; used for generating absolute URLs
root_dir = "/" # the root directory of the site; for example, if the site is hosted at example.com/notes/, set root_dir = "/notes/"
trailing_slash = true # if true, the final URL of each note will have a trailing slash
```

The configuration file is parsed at the start of the program, and values are used as defaults for the corresponding CLI flags. If `cache_dir` is omitted, Weibian uses a project-specific directory under the system temporary directory for intermediate HTML. By default, the configuration file is looked for in `.wb/config.toml` relative to the project root, but a different path can be specified with `--config-file <PATH>`. Site settings can also be overridden via CLI with `--site-domain`, `--site-root-dir`, and `--trailing-slash <BOOL>`. The settings in the `[site]` section are also passed to the Typst compiler as inputs, with the prefix `wb-` and underscores converted to hyphens (e.g., `site.domain` becomes `wb-domain`).

The `trailing_slash` option will affect how internal links are generated and how the output files are organized. If `trailing_slash` is true, each note will be saved in a subdirectory named after its ID, with an `index.html` file inside (e.g., a note with ID `note-123` will be saved as `dist/note-123/index.html`). If false, each note will be saved directly as an HTML file named after its ID (e.g., `dist/note-123.html`). The `root_dir` setting only affects link generation; it does not change where files are written. Special case: a note with ID `index` is always saved as `dist/index.html` and links to the site root.
]

#tr("wb:20250819T221344", expanded: false)

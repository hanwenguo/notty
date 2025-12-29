# Notty: NOte Taking with TYpst

Notty is a Typst-first note system in the spirit of [Forester](https://www.forester-notes.org/index/index.xml). It compiles Typst notes to HTML, then post-processes the HTML to resolve transclusions, internal links, and backmatter (backlinks/contexts).

There is a [demo site](https://hanwenguo.github.io/notty/) showcasing Notty's features, built with Notty itself as a live example.

## Requirements

- Typst CLI available on your PATH (`typst`)
- Rust toolchain only if building from source

## Installation

### Install from source (local checkout)

```bash
cargo install --path .
```

### Install from Git

```bash
cargo install --git https://github.com/hanwenguo/notty
```

### Download a release binary

If a release is available for your platform, download it from:

```text
https://github.com/hanwenguo/notty/releases
```

Place the binary on your PATH.

## Quick start

Using the installed binary:

```bash
notty compile
```

By default, Notty reads Typst sources from `typ/`, uses `.notty/cache` for intermediate HTML, copies assets from `public/`, and outputs the final site to `dist/`. Override paths if needed:

```bash
notty compile \
  --input typ \
  --cache-dir .notty/cache \
  --public-dir public \
  --output dist
```

## Features

- Utilizes Typst HTML export: just use your templates/styles
- Transclusion of notes
- Backmatter generation (backlinks and contexts)
- TOC generation

## Planned

- Bibliography support; most importantly, the References backmatter section

Notty plans to release v1.0 after implementing the above feature. 

- Customizable post-processing hooks
- Datalog-based querying of notes
- Generalized backmatter

After the above features are implemented, Notty will be feature-comparable to Forester. There are also some other nice-to-have features:

- Out-of-the-box dark mode support
- Watch mode for live updates
- Pre-processing hook on source files (this can enable generating PDFs, for example)
- More templating support
- Parallel processing of notes

## Differences from Simalar Projects

### [Forester](https://www.forester-notes.org/index/index.xml)

Notty is following the spirit of Forester. Main differences are now:

- Forester uses its own markup language and LaTeX/KaTeX for math, and Notty uses Typst for everything.
- Forester, as for now, is more mature than Notty.
- Forester now generates XML and Notty generates HTML.

### [Kodama](https://github.com/kokic/kodama)

Kodama is a similar project that uses Typst and Markdown to manage notes. The main difference is that Kodama uses Markdown as the primary note format with good Typst support, while Notty uses Typst as the only note format.

### [Typsite](https://github.com/Glomzzz/typsite)

Typsite is a project that uses Typst to generate static sites. It is very similar to Notty in that they both generate a static site from a collection of Typst files. However, Typsite aims to be a general purpose static site generator, and provides many features (e.g. schema, rewriting, etc.) for that. Notty is more focused on being a tool for taking scientific notes, thus more opinionated and less flexible.

## License

This project is licensed under the GNU General Public License v3.0.

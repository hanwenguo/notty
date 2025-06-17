# Notty: NOte Taking with TYpst

This is a working example for taking notes in the spirit of [Forester](https://www.forester-notes.org/index/index.xml) in [Typst](https://github.com/typst/typst).

## Usage

You need to have [Typst](https://github.com/typst/typst) and [uv](https://docs.astral.sh/uv/) and [rg](https://github.com/BurntSushi/ripgrep) installed.

```bash
chmod +x build.py
./build.py --help
```

## Features

- [x] Transclusion of notes
- [x] Export to PDF and HTML
- [x] Integretation with [Emacs denote package](https://protesilaos.com/emacs/denote)
- [x] Dispatch metadata processing by taxonomy
- [x] Backmatters: backlinks, contexts, etc.

## TODO
- [ ] Better looking for backmatters
- [ ] TOC
- [ ] Bibliography support
    - [ ] Generate nodes for references
    - [ ] Reference section in backmatter
- [ ] Multiple authors & contributors
- [ ] Multilingual support
- [ ] Parallel building of notes
- [ ] Maybe in the future rewrite in Rust and use Typst as a library to be more flexible

## Differences from Simalar Projects

### [Forester](https://www.forester-notes.org/index/index.xml)

Notty is following the spirit of Forester. Main differences are now:

- Forester uses its own markup language, and Notty uses Typst.
- Forester, as for now, is way more mature than Notty.
- Forester now generates XML and Notty generates HTML.

### [Typsite](https://github.com/Glomzzz/typsite)

Typsite is a project that uses Typst to generate static sites. It is very similar to Notty in that they both generate a static site from a collection of Typst files.

Main differences are:

- Typsite aims to be a general purpose static site generator, and provides many features (e.g. schema, rewriting, etc.) for that. Notty is more focused on being a tool for taking scientific notes, thus more opinionated and less flexible. For example, Notty (will) support generating notes directly from BibLaTeX files.

### [Kodama](https://github.com/kokic/kodama)

Kodama is a similar project that uses Typst and Markdown to manage notes. The main difference is that Kodama uses Markdown as the primary note format, while Notty uses Typst.
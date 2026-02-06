# Weibian: A Note System Powered by Typst

Weibian[^1] is a software for taking scientific notes in the spirit of [Forester](https://www.forester-notes.org/index/index.xml), using [Typst](https://typst.app/) as the markup language. It compiles Typst notes to HTML, then post-processes the HTML to resolve transclusion, internal links, and backmatter (backlinks/contexts/references/related notes).

There is a [demo site](https://hanwenguo.github.io/weibian/) showcasing Weibian's features, built with Weibian itself as a live example.

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
cargo install --locked --git https://github.com/hanwenguo/weibian.git
```

Note that the binary will be named `wb`.

### Download a release binary

If a release is available for your platform, download it from:

```text
https://github.com/hanwenguo/weibian/releases
```

Place the binary on your PATH.

## Quick start

Using the installed binary:

```bash
wb compile
```

By default, Weibian reads Typst sources from `typ/`, compiles HTML in-memory, copies assets from `public/`, and outputs the final site to `dist/`. Override paths if needed:

```bash
wb compile \
  --input typ \
  --public-dir public \
  --output dist
```

## Features

- Utilizes Typst HTML export: just use your templates/styles
- Out-of-the-box dark mode support
- Transclusion of notes
- Backmatter generation (backlinks, contexts, references, related notes)
- TOC generation
- PDF export of individual notes
- Include/exclude notes based on glob patterns

## Planned

- Customizable post-processing hooks
- Datalog-based querying of notes
- Generalized backmatter

After the above features are implemented, Weibian will be feature-comparable to Forester. There are also some other nice-to-have features:

- Watch mode
- More templating support
- Parallel processing of notes

These will be on the roadmap for version 2.0.

## Differences from Similar Projects

### [Forester](https://www.forester-notes.org/index/index.xml)

Weibian is following the spirit of Forester. Main differences are now:

- Forester uses its own markup language and LaTeX/KaTeX for math, and Weibian uses Typst for everything.
- Forester, as for now, is more mature than Weibian.
- Forester now generates XML and Weibian generates HTML.

### [Kodama](https://github.com/kokic/kodama)

Kodama is a similar project that uses Typst and Markdown to manage notes. The main difference is that Kodama uses Markdown as the primary note format with good Typst support, while  uses Typst as the only note format.

### [Typsite](https://github.com/Glomzzz/typsite)

Typsite is a project that uses Typst to generate static sites. It is very similar to Weibian in that they both generate a static site from a collection of Typst files. However, Typsite aims to be a general purpose static site generator, and provides many features (e.g. schema, rewriting, etc.) for that. Weibian is more focused on being a tool for taking scientific notes, thus more opinionated and less flexible.

## License

This project is licensed under the GNU General Public License v3.0.

[^1]: Weibian (simplified Chinese: 韦编; traditional Chinese: 韋編; Chinese Pinyin: wéi biān; IPA: [weɪ˧˥ pjɛn˥˥]; roughly “Way-Byen”) means the ancient Chinese technology of bookmaking. Before the invention of paper, Chinese scholars wrote on narrow vertical slips of bamboo or wood (simplified Chinese: 简牍; traditional Chinese: 簡牘) which were then bound together with leather straps to form a book. “韦” (weí) refers to the leather straps used to bind the bamboo slips, and “编” (biān) means to weave, bind, organize or compile. 
Just as leather straps bind scattered bamboo slips into a scroll, this project aims to weave your notes into a structured, cohesive knowledge base. The name is also a tribute to the spirit of learning and researching. According to 《史记》 (_Shiji_, or the _Records of the Grand Historian_), Confucius studied 《易经》(_Yijing_, or _I Ching_, or the _Book of Changes_) with such intensity that the leather bindings of his bamboo book wore out and snapped multiple times. The source text reads: “读《易》，韦编三绝” (“[Confucius] read the _Yijing_ [so frequently that] the leather bindings broke multiple times”). By naming this project Weibian, I honor both the structure of knowledge (the binding) and the persistence required to master it.

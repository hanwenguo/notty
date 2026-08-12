# Weibian: A Note System Powered by Typst

Weibian[^1] is a software for taking scientific notes in the spirit of [Forester](https://www.forester-notes.org/index/index.xml), using [Typst](https://typst.app/) as the markup language. It is built around Typst's experimental bundle export: Weibian discovers notes and assets, generates a synthetic bundle entrypoint, and delegates rendering, links, transclusion, backmatter, and HTML output to Typst templates.

There is a [demo site](https://hanwenguo.github.io/weibian/) showcasing Weibian's features, built with Weibian itself as a live example.

## Requirements

- Rust toolchain only if building from source

Weibian uses Typst as a library by default, so no separate Typst installation is required. If compatibility issues arise, one could switch to a compiler installed on the host system through the `--compiler host` option. Note that the host compiler must be newer than Typst 0.15 (which is in RC stage as of this writing) to work with Weibian.

## Installation

### Homebrew

```bash
brew install hanwenguo/tap/weibian
```

The tap builds Weibian from the tagged source release and provides bottles for
Apple Silicon on macOS 26, and for ARM64 and x86-64 Linux. Intel macOS is not
supported. The installed binary is named `wb`, and Homebrew installs
completions for Bash, Zsh, and Fish.

Other package managers can generate the same scripts with
`wb completions SHELL`.

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

By default, Weibian reads Typst sources from `typ/`, copies bundle assets from `typ/public/`, and outputs the final Typst bundle to `dist/`. Override paths if needed:

```bash
wb compile \
  typ \
  dist \
  --public-dir public \
  --pretty
```

`--public-dir` is resolved relative to the input directory unless it is absolute, so `--public-dir public` means `typ/public`. Include/exclude globs in `weibian.toml` are matched against paths relative to the input directory. If you keep a handwritten bundle entrypoint such as `typ/index.typ`, exclude it explicitly to avoid duplicate `#document` or `#asset` output.

`wb compile` supports most Typst compile options. Weibian owns the input/output paths, project root, output format, and feature flags, and always invokes Typst as bundle export with `bundle,html` enabled.

By default, `wb compile` uses the embedded Typst-as-a-library compiler. To use a host Typst binary instead:

```bash
wb compile --compiler host
```

If the host compiler is not named `typst` or is not on `PATH`, pass its path:

```bash
wb compile --compiler host --host-compiler /path/to/typst
```

You can also set the backend in `weibian.toml`:

```toml
[compiler]
backend = "library" # or "host"
path = "/path/to/typst" # optional, used by the host backend
```

The Cargo features `library-compiler` and `host-compiler` are both enabled by
default. Build with `--no-default-features --features host-compiler` for a
host-only binary, or with `--no-default-features --features library-compiler`
for an embedded-only binary.

## Watch mode

Use `wb watch` or `wb w` while editing notes:

```bash
wb watch
```

Watch mode uses the same input, output, public directory, site, compiler, and
Typst compile flags as `wb compile`. It is currently implemented only for the
embedded library compiler. Running `wb watch --compiler host` exits with a clear
unsupported-backend error.

By default, watch mode writes the Typst bundle to `dist/`, starts a local
live-reload HTTP server, and reloads connected browsers after successful bundle
updates. Server flags mirror Typst watch:

```bash
wb watch --no-serve
wb watch --no-reload
wb watch --port 4000
```

A binary built without the `library-compiler` Cargo feature still parses
`wb watch`, but exits with an error explaining that watch mode requires the
library compiler.

## TFP formula preview server

Weibian can replace the standalone server used by
[tfp.el](https://github.com/hanwenguo/tfp.el) for exact inline Typst formula
previews:

```bash
wb tfp-server --root /absolute/project/root
```

This command implements TFP protocol version 3 over framed JSON-RPC on stdin
and stdout, including live document synchronization, equation discovery,
full-document rendering, diagnostics, SVG metrics, and render-key caching. It
pins the embedded compiler to the Typst 0.15.1 version required by the TFP
client. The TFP root must be a configured Weibian input directory: the server
finds its `weibian.toml` in that directory or an ancestor and always compiles
the same filtered, synthetic bundle entrypoint as `wb compile`. This keeps
HTML/bundle features, cross-note queries, synchronized unsaved sources, public
assets, and `wb-*` site inputs available during formula preview. Protocol
stdout is kept free of logs and diagnostics.

Configure tfp.el with Weibian as the complete server command prefix; TFP
appends `--root` and the canonical project root:

```elisp
(setq tfp-server-command '("wb" "tfp-server"))
```

Every command argument is a separate string and no shell parsing is involved.
Run `M-x tfp-restart-server` after changing the command.
TFP server mode requires the `library-compiler` Cargo feature; a host-only
binary reports that limitation without writing protocol traffic.

## Features

- Utilizes Typst bundle export: just use your Typst templates/styles
- Out-of-the-box dark mode support
- Transclusion of notes
- Backmatter generation (backlinks, contexts, references, related notes)
- TOC generation
- Include/exclude notes based on glob patterns

## Planned

- Datalog-based querying of notes
- Generalized backmatter

After the above features are implemented, Weibian will be feature-comparable to Forester. There are also some other nice-to-have features:

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

Kodama is a similar project that uses Typst and Markdown to manage notes. The main difference is that Kodama uses Markdown as the primary note format with good Typst support, while Weibian uses Typst as the only note format.

### [Typsite](https://github.com/Glomzzz/typsite)

Typsite is a project that uses Typst to generate static sites. It is very similar to Weibian in that they both generate a static site from a collection of Typst files. However, Typsite aims to be a general purpose static site generator, and provides many features (e.g. schema, rewriting, etc.) for that. Weibian is more focused on being a tool for taking scientific notes, thus more opinionated and less flexible.

## License

This project is licensed under the GNU General Public License v3.0.

[^1]: Weibian (simplified Chinese: 韦编; traditional Chinese: 韋編; Chinese Pinyin: wéi biān; IPA: [weɪ˧˥ pjɛn˥˥]; roughly “Way-Byen”) means the ancient Chinese technology of bookmaking. Before the invention of paper, Chinese scholars wrote on narrow vertical slips of bamboo or wood (simplified Chinese: 简牍; traditional Chinese: 簡牘) which were then bound together with leather straps to form a book. “韦” (weí) refers to the leather straps used to bind the bamboo slips, and “编” (biān) means to weave, bind, organize or compile. 
Just as leather straps bind scattered bamboo slips into a scroll, this project aims to weave your notes into a structured, cohesive knowledge base. The name is also a tribute to the spirit of learning and researching. According to 《史记》 (_Shiji_, or the _Records of the Grand Historian_), Confucius studied 《易经》(_Yijing_, or _I Ching_, or the _Book of Changes_) with such intensity that the leather bindings of his bamboo book wore out and snapped multiple times. The source text reads: “读《易》，韦编三绝” (“[Confucius] read the _Yijing_ [so frequently that] the leather bindings broke multiple times”). By naming this project Weibian, I honor both the structure of knowledge (the binding) and the persistence required to master it.

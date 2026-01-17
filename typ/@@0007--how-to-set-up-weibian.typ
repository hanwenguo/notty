#import "/_template/template.typ": template, tr, ln
#show: template(
  title:      [How to set up Weibian],
  date:       datetime(year: 2025, month: 08, day: 19, hour: 22, minute: 12, second: 55),
  tags:       (),
  author: ("hanwenguo",),
  identifier: "0007",
)

Weibian is implemented in Rust. To set up Weibian on your local machine, you can either install using `cargo`:

```sh
cargo install --locked --git https://github.com/hanwenguo/weibian.git
```

Or download the prebuilt binary from the #link("https://github.com/hanwenguo/weibian/releases")[releases page].

Weibian uses a simple project structure to organize your notes and resources. Say you have a directory called `notes/` to store all your notes. Usually, you would want to have a structure like this:

```plain
notes/
├── .wb/        # Weibian configuration and templates
│   ├── config.toml  # Configuration file (optional)
│   ├── templates/   # HTML templates
├── public/        # Resource files to be copied to output directory
│   └── ...
├── dist/          # Default output directory
│   └── ...
└── typ/           # Note files in Typst format
    └── ...
```

Most of the above directories is the default configuration, which can be overridden by passing command line arguments when running Weibian. The `.wb` directory is necessary for now, since it keeps the HTML template files and configuration file (if any). You must create the `.wb` directory manually for now; in the future, Weibian may provide a command to initialize a project structure automatically.

By default, intermediate HTML is cached in a project-specific directory under the system temporary directory. If you prefer a project-local cache, set `cache_dir = ".wb/cache"` in `.wb/config.toml` or pass `--cache-dir .wb/cache`, and create the `.wb/cache/` directory if needed.

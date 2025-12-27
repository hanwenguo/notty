#import "/_template/template.typ": template, tr
#show: template(
  title:      [How to set up Notty],
  date:       datetime(year: 2025, month: 08, day: 19, hour: 22, minute: 12, second: 55),
  tags:       (),
  identifier: "20250819T221255",
)

Notty is implemented in Rust. To set up Notty on your local machine, you can either install using `cargo`:

```sh
cargo install --git https://github.com/hanwenguo/notty
```

Or download the prebuilt binary from the #link("https://github.com/hanwenguo/notty/releases")[releases page].

Notty uses a simple project structure to organize your notes and resources. Say you have a directory called `notes/` to store all your notes. Usually, you would want to have a structure like this:

```plain
notes/
├── .notty/        # Notty configuration and cache
│   ├── config.toml  # Configuration file (optional)
│   ├── templates/   # HTML templates
│   └── cache/      # Cache directory
├── public/        # Resource files to be copied to output directory
│   └── ...
├── dist/          # Default output directory
│   └── ...
└── typ/           # Note files in Typst format
    └── ...
```

Most of the above directories is the default configuration, which can be overridden by passing command line arguments when running Notty. However, the `.notty` directory is necessary for now, since it keeps the HTML template files and configuration file (if any). You must create the `.notty` directory manually for now; in the future, Notty may provide a command to initialize a project structure automatically.

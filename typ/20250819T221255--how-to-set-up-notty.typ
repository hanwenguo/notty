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
├── _template/     # Typst templates of Notty
│   └── ...
├── public/        # Resource files to be copied to output directory
│   └── ...
├── dist/          # Default output directory
│   └── ...
└── typ/           # Note files in Typst format
    └── ...
```

Most of the above directories is the default configuration, which can be overridden by passing command line arguments when running Notty. However, the `_template/` directory is necessary for now, since it keeps the HTML template files used for generating the final HTML files.

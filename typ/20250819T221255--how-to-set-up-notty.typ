#import "/_template/template.typ": template, tr
#show: template(
  title:      [How to set up Notty],
  date:       datetime(year: 2025, month: 08, day: 19, hour: 22, minute: 12, second: 55),
  tags:       (),
  identifier: "20250819T221255",
)

As for now, Notty is essentially the combination of a set of Typst templates and a build script written in Python. So, there is no real installation --- one just clones the #link("https://github.com/hanwenguo/notty")[repository of Notty] as a starter template, edit the site configuration and maybe the template, and write their own notes.

```sh
git clone https://github.com/hanwenguo/notty.git
chmod +x build.py
./build.py --help
```

The build script is intended to be executed directly with execution permission like above instead of by `python build.py`. You must have #link("https://github.com/typst/typst")[Typst], #link("https://docs.astral.sh/uv/")[uv] and #link("https://github.com/BurntSushi/ripgrep")[ripgrep] installed on your machine to run the build script like that.

After cloning, you would see the following structure.

```plain
notty/
├── _template/     # Typst templates of Notty
│   ├── site.typ   # Site configuration
│   └── ...        # Other templates
├── public/        # Resource files to be copied to output directory
│   └── ...
├── html/          # Default output directory for HTML files
│   ├── ...
│   └── pdf/       # Default output directory for PDF files
│       └── ...
├── typ/           # Note files in Typst
│   └── ...
└── build.py       # The build script
```

All of the above directories is the default configuration, which can be overridden by editing the corresponding constants at the beginning of `build.py`. Particularly, the output directories for HTML and PDF files are independent. Thus, for example, you can put them both into a `dist` directory at the same level. That they are nested by default is just because of my personal preference.

You should also look at `site.typ` to check possible configuration options.

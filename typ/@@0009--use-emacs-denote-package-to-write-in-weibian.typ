#import "/_template/template.typ": template, tr, ln
#show: template(
  title:      [Use Emacs denote package to write in Weibian],
  date:       datetime(year: 2025, month: 08, day: 19, hour: 22, minute: 13, second: 44),
  tags:       (),
  author: ("hanwenguo",),
  identifier: "0009",
)

If you use Emacs, Weibian is accompanied by an Emacs Lisp package providing the integration of Weibian and the #link("https://protesilaos.com/emacs/denote")[Denote] package. The following is an example of configuration. However, since everyone has different templates, there's a lot of variables to tweak, and you need to read the source code of the package (it's not very big though) to understand how to customize it. A rewrite of the package to make it more idiomatic is planned.

```lisp
(use-package denote
  :bind (("C-c n n" . denote))
  :config
  (setq denote-directory (expand-file-name "~/Documents/notes/typ/")))

(use-package typst-ts-mode)

(use-package denote-weibian
  :load-path "/path/to/weibian/directory/"
  :after (denote typst-ts-mode)
  :demand t
  :bind (:map typst-ts-mode-map
         ("C-c n b" . denote-weibian-backlinks)
         ("C-c n c" . denote-weibian-contexts)
         ("C-c n t" . denote-weibian-transclude)
         ("C-c n l" . denote-link)
         ("C-c n L" . denote-add-links)
         ("C-c n q c" . denote-query-contents-link) ; create link that triggers a grep
         ("C-c n q f" . denote-query-filenames-link) ; create link that triggers a dired
         ;; Note that `denote-rename-file' can work from any context, not just
         ;; Dired bufffers.  That is why we bind it here to the `global-map'.
         ("C-c n r" . denote-rename-file)
         ("C-c n R" . denote-rename-file-using-front-matter))
  :config
  (push denote-weibian-file-type denote-file-types)

  ;; Customize slugification to allow uppercase letters in signatures; this can also be handled
  ;; at the Typst side.
  (defun +denote-sluggify-signature (str)
    "Make STR an appropriate slug but allowing uppercase letters."
    (denote-slug-put-equals
     (replace-regexp-in-string "[][{}!@#$%^&*()+'\"?,.\|;:~`‘’“”/-]*" "" str)))
  (setq denote-file-name-slug-functions
        '((title . denote-sluggify-title)
          (signature . +denote-sluggify-signature)
          (keyword . denote-sluggify-keyword))))
```

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
  :bind (("C-c n n" . denote)) ;; add your keybindings
  :config
  (setq denote-directory (expand-file-name "~/Documents/notes/typ/"))
  (setq denote-prompts '(signature title))
  (setq denote-excluded-directories-regexp "_template")

  ;;; Use incrementing base-36 numbers as id
  ;;; This function assumes that IDENTIFIERS is a list of base-36 strings
  ;;; i.e. 4-character strings consisting of numbers and uppercase letters
  (defun my/denote-get-next-base36 (identifiers)
    (let ((maxs nil))
      (dolist (s identifiers)
        (let ((u (upcase s)))
          (when (or (null maxs) (string> u maxs))
            (setq maxs u))))
      ;; increment maxs
      (let ((buf (copy-sequence maxs))
            (i 3)
            (carry 1))
        (while (and (>= i 0) (= carry 1))
          (let* ((d (aref buf i)))
            (if (= d ?Z)
                (progn
                  (aset buf i ?0)
                  (setq carry 1))
              (if (= d ?9)
                  (aset buf i ?A)
                (aset buf i (+ d 1)))
              (setq carry 0)))
          (setq i (1- i)))
        buf)))

  (defun my/denote-generate-base36-identifier (initial-identifier _date)
    (let ((denote-used-identifiers (or denote-used-identifiers (denote--get-all-used-ids))))
      (cond (;; Always use the supplied initial-identifier if possible,
             ;; regardless of format.
             (and initial-identifier
                  (not (gethash initial-identifier denote-used-identifiers)))
             initial-identifier)
            (;; Else, the supplied initial-identifier is nil or it is already
             ;; used. Ignore it and generate a valid identifier with the right
             ;; format.
             t
             (let* ((identifiers (hash-table-keys denote-used-identifiers))
                    (case-fold-search nil)
                    (base36-identifiers (seq-filter (lambda (id) (string-match-p "[0-9A-Z]\\{4\\}" id)) identifiers)))
               (if base36-identifiers
                   (my/denote-get-next-base36 base36-identifiers)
                 "0000"))))))
  (setq denote-get-identifier-function #'my/denote-generate-base36-identifier))

(use-package typst-ts-mode)

(use-package denote-weibian
  :load-path "/path/to/weibian/directory/"
  :after (denote)
  :demand t
  :bind (("C-c n b" . denote-weibian-backlinks)
         ("C-c n c" . denote-weibian-contexts)
         ("C-c n t" . denote-weibian-transclude))
  :config
  (push denote-weibian-file-type denote-file-types)

  (setq denote-file-name-slug-functions
        '((title . denote-sluggify-title)
          (signature . identity)
          (keyword . denote-sluggify-keyword))))
```

;;; denote-notty.el --- Extension of denote that integrates with Notty (NOte Taking in TYpst) -*- lexical-binding: t -*-

;; Copyright (C) 2025 Free Software Foundation, Inc.

;; Author: Hanwen Guo <guo@hanwen.io>
;; Maintainer: Hanwen Guo <guo@hanwen.io>
;; URL: https://github.com/hanwenguo/notty
;; Version: 0.1.0
;; Package-Requires: ((emacs "28.1") (denote "4.0.0"))

;; This file is NOT part of GNU Emacs.

;; This program is free software; you can redistribute it and/or modify
;; it under the terms of the GNU General Public License as published by
;; the Free Software Foundation, either version 3 of the License, or
;; (at your option) any later version.
;;
;; This program is distributed in the hope that it will be useful,
;; but WITHOUT ANY WARRANTY; without even the implied warranty of
;; MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
;; GNU General Public License for more details.
;;
;; You should have received a copy of the GNU General Public License
;; along with this program.  If not, see <https://www.gnu.org/licenses/>.

;;; Commentary:
;;
;; Optional extensions for Denote that work specifically with Notty, a note taking system for Typst.

;;; Code:
(require 'denote)

(defvar denote-notty-front-matter
  "#import \"/_template/template.typ\": template, tr
#show: template(
  title:      %s,
  date:       %s,
  tags:       %s,
  identifier: %s,
  taxon:  %s,
)")
(defvar denote-notty-title-key-regexp "^\\s-*title\\s-*:")
(defvar denote-notty-keywords-key-regexp "^\\s-*tags\\s-*:")
(defvar denote-notty-signature-key-regexp "^\\s-*taxon\\s-*:")
(defvar denote-notty-identifier-key-regexp "^\\s-*identifier\\s-*:")
(defvar denote-notty-date-key-regexp "^\\s-*date\\s-*:")

(defun denote-notty--trim-trailing-comma (s)
  "Trim trailing comma from string S."
  (if (string-suffix-p "," s)
   (substring s 0 -1)
  s))

(defun denote-notty--trim-brackets (s)
  "Trim brackets around string S."
  (let ((trims "[][]+"))
    (string-trim s trims trims)))

(defun denote-notty-trim-whitespace-then-comma-then-quotes (s)
  "Trim whitespace then trailing comma then quotes from string S."
  (denote--trim-quotes (denote-trim-whitespace (denote-notty--trim-trailing-comma (denote-trim-whitespace s)))))

(defun denote-notty-trim-whitespace-then-comma-then-brackets (s)
  "Trim whitespace then trailing comma then quotes from string S."
  (denote-notty--trim-brackets (denote-trim-whitespace (denote-notty--trim-trailing-comma (denote-trim-whitespace s)))))

(defun denote-notty-format-string-for-front-matter (s)
  "Surround string S with quotes.

This can be used in `denote-file-types' to format front mattter."
  (let ((completion-ignore-case nil))
    (format "\"%s\"" s)))

(defun denote-notty-format-string-into-content-for-front-matter (s)
  "Surround string S with quotes.

This can be used in `denote-file-types' to format front mattter."
  (let ((completion-ignore-case nil))
    (format "[%s]" s)))

(defun denote-notty-format-keywords-for-front-matter (keywords)
  "Format front matter KEYWORDS for Typst file type.
KEYWORDS is a list of strings.  Consult the `denote-file-types'
for how this is used."
  (format "(%s)" (mapconcat (lambda (k) (format "%S," k)) keywords " ")))

(defun denote-notty-extract-keywords-from-front-matter (keywords-string)
  "Extract keywords list from front matter KEYWORDS-STRING.
Split KEYWORDS-STRING into a list of strings.

Consult the `denote-file-types' for how this is used."
  (split-string keywords-string "[:,\s]+" t "[][)( \"']+"))

(defun denote-notty-format-date (date)
  "Format DATE as Typst datetime."
  (if date
     (format-time-string
      "datetime(year: %Y, month: %m, day: %d, hour: %H, minute: %M, second: %S)"
      date)
    ""))

(defmacro denote-notty--define-retrieve-date (field)
  "Define a function to retrive FIELD of a Typst datetime expression."
  (declare (indent 1))
  `(defun ,(intern (format "denote-notty--retrieve-date-%s" field)) (date-string)
     (string-match ,(format "%s\\s-*:\\s-*\\([[:digit:]]+\\)\\s-*\\(,\\|)\\)" field)
                   date-string)
     (let ((matched (match-string 1 date-string)))
        matched)))

(denote-notty--define-retrieve-date year)
(denote-notty--define-retrieve-date month)
(denote-notty--define-retrieve-date day)
(denote-notty--define-retrieve-date hour)
(denote-notty--define-retrieve-date minute)
(denote-notty--define-retrieve-date second)

(defun denote-notty--extract-date-from-front-matter (date-string)
  "Extract date object from front matter DATE-STRING."
  (let ((year (denote-notty--retrive-date-year date-string))
        (month (denote-notty--retrive-date-month date-string))
        (day (denote-notty--retrive-date-day date-string))
        (hour (denote-notty--retrive-date-hour date-string))
        (minute (denote-notty--retrive-date-minute date-string))
        (second (denote-notty--retrive-date-second date-string)))
    (if (and year month day hour minute second)
       (encode-time
       (string-to-number second)
       (string-to-number minute)
       (string-to-number hour)
       (string-to-number day)
       (string-to-number month)
       (string-to-number year)))))

(defun denote-notty-extract-date-from-front-matter (date-string)
  "Extract date object from front matter DATE-STRING.

Consult the `denote-file-types' for how this is used."
  (let ((date-string (denote-notty-trim-whitespace-then-comma-then-quotes date-string)))
    (if (string-empty-p date-string)
      nil
     (denote-notty--extract-date-from-front-matter date-string))))

(defvar denote-notty-link-format "#ln(\"denote:%s\")[%s]")
(defvar denote-notty-link-in-context-regexp
  "#ln([[:blank:]]*\"denote:\\(?1:[^\"()]+?\\)\"[[:blank:]]*)\\[\\(?2:.*?\\)\\]")
(defvar denote-notty-transclusion-format "#tr(\"denote:%s\")")

(defvar denote-notty-file-type
  `(notty
    :extension ".typ"
    :front-matter denote-notty-front-matter
    :link denote-notty-link-format
    :link-in-context-regexp denote-notty-link-in-context-regexp
    :title-key-regexp ,denote-notty-title-key-regexp
    :title-value-function denote-notty-format-string-into-content-for-front-matter
    :title-value-reverse-function denote-notty-trim-whitespace-then-comma-then-brackets
    :keywords-key-regexp ,denote-notty-keywords-key-regexp
    :keywords-value-function denote-notty-format-keywords-for-front-matter
    :keywords-value-reverse-function denote-notty-extract-keywords-from-front-matter
    :signature-key-regexp ,denote-notty-signature-key-regexp
    :signature-value-function denote-notty-format-string-for-front-matter
    :signature-value-reverse-function denote-notty-trim-whitespace-then-comma-then-quotes
    :identifier-key-regexp ,denote-notty-identifier-key-regexp
    :identifier-value-function denote-notty-format-string-for-front-matter
    :identifier-value-reverse-function denote-notty-trim-whitespace-then-comma-then-quotes
    :date-key-regexp ,denote-notty-date-key-regexp
    :date-value-function denote-notty-format-date
    :date-value-reverse-function denote-notty-extract-date-from-front-matter))

(defun denote-notty-format-transclude (file)
  "Prepare transclusion to FILE."
  (let* ((identifier (denote-retrieve-filename-identifier file)))
    (format
     denote-notty-transclusion-format
     identifier)))

;;;###autoload
(defun denote-notty-transclude (file)
  "Create transclusion to FILE note in variable `denote-directory'.

When called interactively, prompt for FILE using completion.  In this
case, derive FILE-TYPE from the current buffer.  FILE-TYPE is used to
determine the format of the link.

When called from Lisp, FILE is a string representing a full file system
path.  FILE-TYPE is a symbol as described in the user option
`denote-file-type'.  DESCRIPTION is a string.  Whether the caller treats
the active region specially, is up to it."
  (interactive
  (let* ((file (denote-file-prompt nil "Link to FILE")))
    (list file current-prefix-arg)))
  (unless (or (denote--file-type-org-extra-p)
           (and buffer-file-name (denote-file-has-supported-extension-p buffer-file-name)))
  (user-error "The current file type is not recognized by Denote"))
  (unless (file-exists-p file)
    (user-error "The transcluded file does not exist"))
  (insert (denote-format-link file)))

(defun denote-notty-backlinks-query-regexp (id)
  "Return a regexp to query contexts of file with ID."
  (rx
   "#ln("
   (zero-or-more blank)
   "\"denote:"
   (literal id)
   "\""
   (zero-or-more blank)
   ")"))

(defun denote-notty-contexts-query-regexp (id)
  "Return a regexp to query contexts of file with ID."
  (rx
   line-start
   (zero-or-more blank)
   "#tr("
   (zero-or-more blank)
   "\"denote:"
   (literal id)
   "\""))

(defun denote-notty--contexts-get-buffer-name (file id)
  "Format a buffer name for `denote-notty-contexts'.
Use FILE to detect a suitable title with which to name the buffer.  Else
use the ID."
  (denote-format-buffer-name
  (if-let* ((type (denote-filetype-heuristics file))
              (title (denote-retrieve-front-matter-title-value file type)))
        (format "FILE contexts for %S" title)
    (format "FILE contexts for %s" id))
   :special-buffer))

;;;###autoload
(defun denote-notty-contexts ()
  "Produce a buffer with contexts to the current note.

By contexts, one mean files transcluding the current note. Show the
names of files linking to the current file. Include the content of each
context if the user option `denote-notty-contexts-show-content' is non-nil.

Place the buffer below the current window or wherever the user option
`denote-notty-contexts-display-buffer-action' specifies."
  (interactive)
  (if-let* ((file buffer-file-name))
   (when-let* ((identifier (denote-retrieve-filename-identifier-with-error file))
               (query (denote-notty-contexts-query-regexp identifier)))
  (funcall denote-query-links-buffer-function
       query nil
       (denote-notty--contexts-get-buffer-name file identifier)
       denote-backlinks-display-buffer-action))
    (user-error "Buffer `%s' is not associated with a file" (current-buffer))))

(defalias 'denote-notty-show-contexts-buffer 'denote-notty-contexts
  "Alias for `denote-notty-contexts' command.")

(defun denote-notty-get-contexts (&optional file)
  "Return list of contexts in current or optional FILE.
Also see `denote-link-return-backlinks'."
  (when-let* ((current-file (or file (buffer-file-name)))
              (id (denote-retrieve-filename-identifier-with-error current-file)))
    (delete current-file (denote-retrieve-files-xref-query
                      (denote-notty-contexts-query-regexp id)))))

(defun denote-notty--file-has-contexts-p (file)
  "Return non-nil if FILE has contexts."
  (not (zerop (length (denote-notty-link-return-contexts file)))))

;;;###autoload
(defun denote-notty-find-context ()
  "Use minibuffer completion to visit context to current file.
Alo see `denote-find-backlink'."
  (declare (interactive-only t))
  (interactive)
  (find-file
  (denote-get-path-by-id
   (denote-extract-id-from-string
    (denote-select-linked-file-prompt
       (or (denote-notty-get-contexts)
           (user-error "No context found")))))))
;;;###autoload
(defun denote-notty-find-context ()
  "Use minibuffer completion to visit transcluding parent to current file.
Visit the file itself, not the location where the link is.  For a
context-sensitive operation, use `denote-notty-find-context-with-location'.

Alo see `denote-find-link'."
  (declare (interactive-only t))
  (interactive)
  (when-let* ((links (or (denote-notty-get-contexts)
                         (user-error "No contexts found")))
              (selected (denote-select-from-files-prompt links "Select among CONTEXTS")))
    (find-file selected)))

;;;###autoload
(defun denote-notty-find-context-with-location ()
  "Like `denote-find-backlink' but jump to the exact location of the link."
  (declare (interactive-only t))
  (interactive)
  (when-let* ((current-file buffer-file-name)
              (id (denote-retrieve-filename-identifier-with-error current-file))
              (query (denote-notty-contexts-query-regexp id))
              (files (denote-directory-files nil :omit-current :text-only))
              (fetcher (lambda () (xref-matches-in-files query files))))
    (xref-show-definitions-completing-read fetcher nil)))

;;;###autoload
(defun denote-notty-backlinks ()
  "Produce a buffer with backlinks to the current note.

Show the names of files linking to the current file.  Include the
context of each link if the user option `denote-backlinks-show-context'
is non-nil.

Place the buffer below the current window or wherever the user option
`denote-backlinks-display-buffer-action' specifies."
  (interactive)
  (if-let* ((file buffer-file-name))
   (when-let* ((identifier (denote-retrieve-filename-identifier-with-error file))
              (query (denote-notty-backlinks-query-regexp identifier)))
  (funcall denote-query-links-buffer-function
        query nil
        (denote--backlinks-get-buffer-name file identifier)
        denote-backlinks-display-buffer-action))
    (user-error "Buffer `%s' is not associated with a file" (current-buffer))))

(defun denote-notty-get-backlinks (&optional file)
  "Return list of backlinks in current or optional FILE.
Also see `denote-get-links'."
  (when-let* ((current-file (or file (buffer-file-name)))
              (id (denote-retrieve-filename-identifier-with-error current-file)))
    (delete current-file (denote-retrieve-files-xref-query
                      (denote-notty-backlinks-query-regexp id)))))

;;;###autoload
(defun denote-notty-find-backlink ()
  "Use minibuffer completion to visit backlink to current file.
Visit the file itself, not the location where the link is.  For a
context-sensitive operation, use `denote-find-backlink-with-location'.

Alo see `denote-find-link'."
  (declare (interactive-only t))
  (interactive)
  (when-let* ((links (or (denote-notty-get-backlinks)
                         (user-error "No backlinks found")))
              (selected (denote-select-from-files-prompt links "Select among BACKLINKS")))
    (find-file selected)))

;;;###autoload
(defun denote-notty-find-backlink-with-location ()
  "Like `denote-find-backlink' but jump to the exact location of the link."
  (declare (interactive-only t))
  (interactive)
  (when-let* ((current-file buffer-file-name)
              (id (denote-retrieve-filename-identifier-with-error current-file))
              (query (denote-notty-backlinks-query-regexp id))
              (files (denote-directory-files nil :omit-current :text-only))
              (fetcher (lambda () (xref-matches-in-files query files))))
    (xref-show-definitions-completing-read fetcher nil)))

(provide 'denote-notty)
;;; denote-notty.el ends here

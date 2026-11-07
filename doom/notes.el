;;; $DOOMDIR/notes.el -*- lexical-binding: t; -*-

(require 'denote)

(defun pg/denote-project-template ()
  (insert "#+BEGIN: pg-tasks :project-name "
          (denote-sluggify-keyword (denote-retrieve-filename-title buffer-file-name))
          "\n"
          "#+END:")
  (org-update-all-dblocks)
  "")
(add-to-list 'denote-templates '(pg-project . pg/denote-project-template))

(defun pg/denote-find-project-files (project-name)
  "Find all notes related to PROJECT-NAME."
  (seq-filter (lambda (file)
                (and
                 ;; Find only project files
                 (string-match "_project" file)
                 ;; And everything tagged with project name
                 (string-match (concat "_" (denote-sluggify-keyword project-name)) file)))
              (denote-directory-files)))

(defun org-dblock-write:pg-tasks (params)
  (let* ((project-name (symbol-name (plist-get params :project-name)))
         ;; First, find ALL notes related to this project.
         (files (pg/denote-find-project-files project-name))
         (files (seq-filter (lambda (file)
                              (and
                               ;; Then filter for tasks.
                               (string-match "_task" file)
                               ;; But are not done
                               (not (string-match "_done" file))
                               ;; or cancelled
                               (not (string-match "_cancelled" file))))
                            files)))
    (denote-link--insert-links files 'org nil t)))

(defun pg/denote-task-toggle-done ()
  (interactive)
  (when (denote-file-is-note-p buffer-file-name)
    (let ((denote-rename-confirmations nil)
          (keywords (denote-extract-keywords-from-path buffer-file-name)))
      (setq keywords (remove "cancelled" keywords))
      (if (member "done" keywords)
          (setq keywords (remove "done" keywords))
        (setq keywords (append keywords '("done"))))
      (denote-rename-file buffer-file-name
                          'keep-current
                          keywords
                          'keep-current
                          'keep-current
                          'keep-current))))

(defun pg/denote-task-toggle-cancelled ()
  (interactive)
  (when (denote-file-is-note-p buffer-file-name)
    (let ((denote-rename-confirmations nil)
          (keywords (denote-extract-keywords-from-path buffer-file-name)))
      (setq keywords (remove "done" keywords))
      (if (member "cancelled" keywords)
          (setq keywords (remove "cancelled" keywords))
        (setq keywords (append keywords '("cancelled"))))
      (denote-rename-file buffer-file-name
                          'keep-current
                          keywords
                          'keep-current
                          'keep-current
                          'keep-current))))

(setq denote-directory "~/.notes/")
(setq denote-prompts '(title keywords))
(add-hook! 'dired-mode #'denote-dired-mode-in-directories)
(map! :leader "n d" #'denote)

(after! treebundel
  (defun pg/denote-task-capture ()
    "Create a new task for the current or specified treebundel
workspace."
    (interactive)
    (let ((workspace (or (when current-prefix-arg (treebundel-read-workspace))
                         (treebundel-current-workspace)
                         (treebundel-read-workspace)))
          (denote-title-history nil))
      (denote (denote-title-prompt nil "New task")
              `(,workspace "task" "project"))))

  (transient-define-suffix pg/treebundel-project-notes (workspace project)
    "Open the notes for a treebundel project."
    :if (lambda () (treebundel-scope-project-p (transient-scope)))
    (interactive (if (treebundel-scope-project-p (transient-scope))
                     (list (oref (transient-scope) workspace) (oref (transient-scope) project))
                   (let* ((workspace (treebundel-read-workspace))
                          (project (treebundel-read-project workspace)))
                     (list workspace project))))
    ;; Find the WORKSPACE as a tag instead
    (if-let ((project-notes (seq-find (lambda (file)
                                        (string-match (concat "--" (denote-sluggify-title workspace)) file))
                                      (pg/denote-find-project-files workspace))))
        ;; Open it if it exists
        (find-file-other-window project-notes)
      ;; Otherwise, create a new one with the new template
      (denote workspace `(,workspace "project") nil nil nil 'pg-project)))

  (transient-append-suffix 'treebundel "a"
    '("x" "Notes" pg/treebundel-project-notes) :keep-other))

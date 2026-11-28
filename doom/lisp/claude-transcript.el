;;; claude-transcript.el --- Moved to Homun; this loads its copy -*- lexical-binding: t; -*-

;; The transcript and card views live in the Homun checkout now (container/emacs).
;; A cockpit started by a launcher from before the move still looks for them here:
;; this loads Homun's copy, and goes once no host runs such a launcher.

(let ((dir (seq-find (lambda (dir) (file-exists-p (expand-file-name "claude-transcript.el" dir)))
                     (cons (or (getenv "CLAUDE_EMACS_CONF") "/opt/homun/emacs")
                           (file-expand-wildcards "~/claude/_tools/*/container/emacs")))))
  (unless dir (error "claude-transcript: no Homun checkout with container/emacs here"))
  ;; Appended: only what is missing before it (the card) is taken from there
  (add-to-list 'load-path dir t)
  (load (expand-file-name "claude-transcript.el" dir) nil t))

;;; claude-transcript.el ends here

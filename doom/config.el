;;; $DOOMDIR/config.el -*- lexical-binding: t; -*-

(use-package! kdl-mode
  :defer t
  :config
  ;; Disable kdl-mode automatically turning on because it causes hard freeze of Emacs.
  (setq auto-mode-alist (remove '("\\.kdl\\'" . kdl-mode) auto-mode-alist)))

;; Place your private configuration here! Remember, you do not need to run 'doom
;; sync' after modifying this file!

;; Some functionality uses this to identify you, e.g. GPG configuration, email
;; clients, file templates and snippets. It is optional.
;; (setq user-full-name "John Doe"
;;       user-mail-address "john@doe.com")

;; Doom exposes five (optional) variables for controlling fonts in Doom:
;;
;; - `doom-font' -- the primary font to use
;; - `doom-variable-pitch-font' -- a non-monospace font (where applicable)
;; - `doom-big-font' -- used for `doom-big-font-mode'; use this for
;;   presentations or streaming.
;; - `doom-symbol-font' -- for symbols
;; - `doom-serif-font' -- for the `fixed-pitch-serif' face
;;
;; See 'C-h v doom-font' for documentation and more examples of what they
;; accept. For example:
;;
;;(setq doom-font (font-spec :family "Fira Code" :size 12 :weight 'semi-light)
;;      doom-variable-pitch-font (font-spec :family "Fira Sans" :size 13))
;;
;; If you or Emacs can't find your font, use 'M-x describe-font' to look them
;; up, `M-x eval-region' to execute elisp code, and 'M-x doom/reload-font' to
;; refresh your font settings. If Emacs still can't find your font, it likely
;; wasn't installed correctly. Font issues are rarely Doom issues!

;; There are two ways to load a theme. Both assume the theme is installed and
;; available. You can either set `doom-theme' or manually load a theme with the
;; `load-theme' function. This is the default:
(setq doom-theme 'modus-vivendi)

;; This determines the style of line numbers in effect. If set to `nil', line
;; numbers are disabled. For relative line numbers, set this to `relative'.
(setq display-line-numbers-type 'relative)
(setq scroll-margin 10) ;; Scroll buffer when point is 15 lines from edge of window

;; Wrap long lines; truncating is opt-in per buffer with `toggle-truncate-lines'
(setq-default truncate-lines nil)
(setq org-startup-truncated nil)

;; If you use `org' and don't want your org files in the default location below,
;; change `org-directory'. It must be set before org loads!
(setq org-directory "~/.org/")


;; Whenever you reconfigure a package, make sure to wrap your config in an
;; `with-eval-after-load' block, otherwise Doom's defaults may override your
;; settings. E.g.
;;
;;   (with-eval-after-load 'PACKAGE
;;     (setq x y))
;;
;; The exceptions to this rule:
;;
;;   - Setting file/directory variables (like `org-directory')
;;   - Setting variables which explicitly tell you to set them before their
;;     package is loaded (see 'C-h v VARIABLE' to look them up).
;;   - Setting doom variables (which start with 'doom-' or '+').
;;
;; Here are some additional functions/macros that will help you configure Doom.
;;
;; - `load!' for loading external *.el files relative to this one
;; - `add-load-path!' for adding directories to the `load-path', relative to
;;   this file. Emacs searches the `load-path' when you load packages with
;;   `require' or `use-package'.
;; - `map!' for binding new keys
;;
;; To get information about any of these functions/macros, move the cursor over
;; the highlighted symbol at press 'K' (non-evil users must press 'C-c c k').
;; This will open documentation for it, including demos of how they are used.
;; Alternatively, use `C-h o' to look up a symbol (functions, variables, faces,
;; etc).
;;
;; You can also try 'gd' (or 'C-c c d') to jump to their definition and see how
;; they are implemented.

;; (load! "~/workspaces/treebundel/treebund.el/treebundel.el")

;;; load! af
(load! "keybinds.el")
(load! "notes.el")
(when (daemonp) (require 'magit))

;;; treebundel
(use-package! treebundel
  :init
  (map! :leader :desc "Treebundel" "p" #'treebundel)
  :config
  (defun pg/treebundel-magit (workspace project)
    "Open magit in WORKSPACE/PROJECT."
    (interactive (when-let* ((workspace (car (transient-scope transient-current-command)))
                             (project (cdr (transient-scope transient-current-command))))
                   (list workspace project)))
    (let ((magit-display-buffer-function #'magit-display-buffer-fullframe-status-v1))
      (magit-status-setup-buffer (treebundel-project-path workspace project))))

  (defun pg/opened-new-workspace ()
    (find-file (treebundel-workspace-path (treebundel-current-workspace))))

  (defun pg/open-new-project (project-path)
    (when-let* ((workspace (treebundel-current-workspace project-path))
                (project (treebundel--project-of project-path)))
      (require 'magit nil t)
      (let ((magit-display-buffer-function #'magit-display-buffer-fullframe-status-v1))
        (magit-status-setup-buffer (treebundel-project-path workspace project)))))
  (setopt treebundel-project-open-function #'pg/open-new-project))

;;; Unbind weird emacs defaults
(map! :map (global-map help-quick-use-map) "TAB" nil)

;;; puni
(use-package! puni
  :init
  (defun pg/puni-change-line ()
    (interactive)
    (call-interactively #'puni-kill-line)
    (evil-insert 0))

  (puni-global-mode 1)

  :config
  (unbind-key (kbd "C-k") puni-mode-map)
  (evil-define-key* 'normal puni-mode-map
    (kbd "C") #'pg/puni-change-line
    (kbd "D") #'puni-kill-line)

  ;; Interferes with keybind some vertico menus
  (after! vertico
    (add-hook 'vertico-mode-hook (lambda () (puni-mode 0)))))

;;; share-path
(use-package share-path
  :init
  (share-path-mode 1)
  (after! org (add-to-list 'share-path-ignore-dirs org-directory))
  (after! denote (add-to-list 'share-path-ignore-dirs denote-directory)))

;;; Flycheck
(after! flycheck
  (evil-define-key* 'normal flycheck-mode-map
    (kbd "C-k") #'("prev error" . flycheck-previous-error)
    (kbd "C-j") #'("next error" . flycheck-next-error)))

;;; Tree
(after! (nerd-icons neotree)
  (setq neo-theme 'nerd-icons))

(use-package! outli
  :init
  (map! :n "<tab>" nil) ;; Allow heading toggle with TAB
  (map! :m "<tab>" nil) ;; Allow heading toggle with TAB
  (add-hook! 'emacs-lisp-mode-hook #'outli-mode))

(use-package! hass
  :defer 60
  :init
  ;; hass-host is machine-specific: set it in local.el
  (setq hass-insecure t)
  (setq hass--debug t)
  (setq hass--debug-ignore '("EVENT" "HTTP"))

  (defun pg/hass-call-service ()
    (interactive)
    (map! :n "M-a" #'hass-call-service)
    (message "Connected to hass")
    (remove-hook 'hass-loaded-hook #'pg/hass-call-service)
    (call-interactively #'hass-call-service))

  (map! :n "M-a"
        (lambda () (interactive)
          (add-hook 'hass-loaded-hook #'pg/hass-call-service)
          (message "Connecting to hass...")
          (setq hass-apikey (auth-source-pass-get 'secret "home/hass/emacs-apikey"))
          (hass-mode 1)))

  ;; (setq hass-dash-layouts (concat (file-name-concat (file-truename (expand-file-name user-emacs-directory)))))

  :config
  (hass-mode 1))

(use-package! hass-dash
  :defer t)

(after! eglot
  (defun pg/eglot-managed-mode-hook ()
    (eglot-inlay-hints-mode -1))
  (add-hook! 'eglot-managed-mode-hook #'pg/eglot-managed-mode-hook))

(remove-hook 'savehist-additional-variables 'kill-ring)

(use-package! nushell-mode
  :if (executable-find "nufmt")
  :init
  (after! apheleia
    (add-to-list 'apheleia-formatters '(nufmt "nufmt" "--stdin"))))

;;; Local (machine-specific, gitignored, see local.el.example)
(load! "local" nil t)

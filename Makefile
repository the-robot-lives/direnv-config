DC_HOME      := $(CURDIR)
INSTALL_DIR  := $(HOME)/.local/bin
DIRENV_LIB   := $(HOME)/.config/direnv/lib
ZSHRC        := $(HOME)/.zshrc
STATE_DIR    := $(HOME)/.local/state/direnv-config
# Hook references the installed dc-init (stable path), never the source checkout,
# so moving/remounting the checkout can't break the shell hook.
SHELL_INIT   := eval "$$($(INSTALL_DIR)/dc-init zsh)"
CARGO        := cargo
RELEASE_BIN  := target/release/dc

.PHONY: compile build test install uninstall check doctor help clean
.PHONY: install-direnv-lib install-shell-hook install-cli install-completions
.PHONY: sdk-test sdk-build sdk-publish sdk-public sdk-clean

help:
	@echo "Targets:"
	@echo "  compile    Build the dc binary (cargo build --release)"
	@echo "  build      Alias for compile"
	@echo "  test       Run Rust tests + integration tests"
	@echo "  install    Build + install binary, direnv stdlib, shell hook"
	@echo "  uninstall  Remove all installed components"
	@echo "  check      Verify installation"
	@echo "  doctor     Diagnose common issues"
	@echo "  clean      Remove build artifacts"
	@echo ""
	@echo "SDK targets (fan out to sdk/{elixir,php,python,rust,typescript}):"
	@echo "  sdk-test     Run tests for all SDKs"
	@echo "  sdk-build    Build all SDKs"
	@echo "  sdk-publish  Publish all SDKs"
	@echo "  sdk-clean    Clean all SDK build artifacts"

# --- Build ---

compile:
	@if [ -f Cargo.toml ]; then \
		echo "==> Building dc binary (release)"; \
		if command -v $(CARGO) >/dev/null 2>&1; then \
			$(CARGO) build --release; \
		else \
			echo "==> Skipping compile: cargo not found."; \
		fi; \
	else \
		echo "==> Cargo.toml not yet created — skipping compile"; \
	fi

build: compile

# --- Test ---

test:
	@echo "==> Running direnv-config tests"
	@if [ -f Cargo.toml ]; then \
		if command -v $(CARGO) >/dev/null 2>&1; then \
			$(CARGO) test; \
		else \
			echo "==> Skipping tests: cargo not found."; \
		fi; \
	else \
		echo "    Cargo.toml not yet created — skipping Rust tests"; \
	fi
	@if [ -d tests/integration ]; then \
		echo "==> Running integration tests"; \
		for t in tests/integration/*.sh; do \
			echo "--- $$(basename $$t) ---"; \
			sh "$$t" || exit 1; \
		done; \
		echo "==> Integration tests passed"; \
	fi

# --- Install ---

install: compile install-direnv-lib install-shell-hook install-cli
	@$(MAKE) install-completions
	@echo ""
	@echo "==> direnv-config installed"
	@echo "    1. dc binary      → $(INSTALL_DIR)/dc"
	@echo "    2. direnv stdlib  → $(DIRENV_LIB)/dc.sh"
	@echo "    3. shell hook     → $(ZSHRC) (dc-init zsh)"
	@echo ""
	@echo "    Run: source ~/.zshrc  (or open a new shell)"

install-direnv-lib:
	@echo "==> Installing direnv stdlib extension"
	@mkdir -p $(DIRENV_LIB)
	@# Copy (not symlink): a symlink into the checkout dangles whenever the checkout
	@# moves or its volume remounts. rm first so an old symlink is replaced, not
	@# written through. Re-run this target after editing lib/direnv-stdlib.sh.
	@rm -f $(DIRENV_LIB)/dc.sh
	@install -m 644 $(DC_HOME)/lib/direnv-stdlib.sh $(DIRENV_LIB)/dc.sh
	@echo "    $(DC_HOME)/lib/direnv-stdlib.sh → $(DIRENV_LIB)/dc.sh (copy)"

# Adds the dc-init hook to ZSHRC. Existing hook lines whose dc-init path does not
# resolve (stale absolute paths) are rewritten in place after a timestamped backup;
# valid existing lines are left untouched.
install-shell-hook:
	@echo "==> Installing shell hook"
	@touch $(ZSHRC); \
	stale=""; valid=false; \
	for n in $$(grep -n 'dc-init' $(ZSHRC) | grep -v '^[0-9]*:[[:space:]]*#' | cut -d: -f1); do \
		cmd=$$(sed -n "$${n}p" $(ZSHRC) | sed -E 's/.*\$$\(([^ )]*dc-init).*/\1/'); \
		case "$$cmd" in \
			*/*) if [ -x "$$cmd" ]; then valid=true; else stale="$$stale $$n"; fi ;; \
			*)   if [ -x "$(INSTALL_DIR)/dc-init" ] || command -v "$$cmd" >/dev/null 2>&1; then valid=true; else stale="$$stale $$n"; fi ;; \
		esac; \
	done; \
	if [ -n "$$stale" ]; then \
		bak="$(ZSHRC).bak.dc-$$(date +%Y%m%d%H%M%S)"; \
		cp $(ZSHRC) "$$bak"; \
		for n in $$stale; do \
			echo "    ~ $(ZSHRC):$$n (stale dc-init path)"; \
			echo "      - $$(sed -n "$${n}p" $(ZSHRC))"; \
			echo '      + $(SHELL_INIT)'; \
		done; \
		awk -v lines="$$stale" -v repl='$(SHELL_INIT)' \
			'BEGIN { n = split(lines, a, " "); for (i = 1; i <= n; i++) fix[a[i]] = 1 } \
			 (NR in fix) { print repl; next } { print }' "$$bak" > $(ZSHRC).dc-tmp && \
		cat $(ZSHRC).dc-tmp > $(ZSHRC) && rm -f $(ZSHRC).dc-tmp; \
		echo "    ✓ Rewrote stale dc-init line(s); backup: $$bak"; \
	elif $$valid; then \
		echo "    ✓ Already present in $(ZSHRC)"; \
	else \
		echo '' >> $(ZSHRC); \
		echo '# direnv-config: IPC hook (version watcher + tabbing bridge)' >> $(ZSHRC); \
		echo '$(SHELL_INIT)' >> $(ZSHRC); \
		echo "    ✓ Added dc-init to $(ZSHRC)"; \
	fi

install-cli:
	@echo "==> Installing CLI binary"
	@mkdir -p $(INSTALL_DIR)
	@if [ -f "$(RELEASE_BIN)" ]; then \
		src=$$(realpath "$(RELEASE_BIN)"); dst=$$(realpath "$(INSTALL_DIR)/dc" 2>/dev/null); \
		if [ "$$src" = "$$dst" ]; then \
			echo "    dc: same file — skipping"; \
		else \
			install -m 755 "$(RELEASE_BIN)" "$(INSTALL_DIR)/dc"; \
		fi; \
		echo "    $(INSTALL_DIR)/dc ($$($(RELEASE_BIN) --version 2>/dev/null || echo 'built'))"; \
	else \
		echo "    ⚠ Binary not found — run 'make compile' first"; \
	fi
	@if [ -f bin/dc-init ]; then \
		src=$$(realpath bin/dc-init); dst=$$(realpath "$(INSTALL_DIR)/dc-init" 2>/dev/null); \
		if [ "$$src" = "$$dst" ]; then \
			echo "    dc-init: same file — skipping"; \
		else \
			install -m 755 bin/dc-init "$(INSTALL_DIR)/dc-init"; \
		fi; \
		echo "    $(INSTALL_DIR)/dc-init"; \
	fi
	@if [ -f bin/tabbing-on-step ]; then \
		src=$$(realpath bin/tabbing-on-step); dst=$$(realpath "$(INSTALL_DIR)/tabbing-on-step" 2>/dev/null); \
		if [ "$$src" = "$$dst" ]; then \
			echo "    tabbing-on-step: same file — skipping"; \
		else \
			install -m 755 bin/tabbing-on-step "$(INSTALL_DIR)/tabbing-on-step"; \
		fi; \
		echo "    $(INSTALL_DIR)/tabbing-on-step"; \
	fi

install-completions:
	@DATA_DIR="$${XDG_DATA_HOME:-$$HOME/.local/share}"; \
	BASH_DIR="$$DATA_DIR/bash-completion/completions"; \
	ZSH_DIR="$$DATA_DIR/zsh/site-functions"; \
	if ! mkdir -p "$$BASH_DIR" "$$ZSH_DIR" 2>/dev/null; then \
		echo "dc: cannot write completion dirs; skipping."; \
		exit 0; \
	fi; \
	cp completions/dc.bash "$$BASH_DIR/dc"; \
	cp completions/_dc "$$ZSH_DIR/_dc"; \
	echo "dc: completions installed (bash-completion + zsh)"; \
	if ! grep -qs "zsh/site-functions" "$$HOME/.zshrc" 2>/dev/null; then \
		echo "dc: zsh users — add to .zshrc before compinit:"; \
		echo "  fpath=($$ZSH_DIR \$$fpath)"; \
	fi

# --- Uninstall ---

uninstall:
	@echo "==> Removing direnv stdlib extension"
	@rm -f $(DIRENV_LIB)/dc.sh
	@echo "==> Removing CLI"
	@rm -f $(INSTALL_DIR)/dc $(INSTALL_DIR)/dc-init
	@echo "==> NOTE: Shell hook line left in $(ZSHRC) — remove manually:"
	@echo '    $(SHELL_INIT)'
	@echo "==> NOTE: State directory left intact: $(STATE_DIR)"
	@echo "    Remove with: rm -rf $(STATE_DIR)"

# --- Diagnostics ---

check:
	@echo "==> Checking direnv-config installation"
	@ok=true; \
	if [ -x "$(INSTALL_DIR)/dc" ]; then \
		ver=$$("$(INSTALL_DIR)/dc" --version 2>/dev/null || echo "unknown"); \
		echo "  ✓ dc binary: $(INSTALL_DIR)/dc ($$ver)"; \
	else \
		echo "  ✗ dc binary: not found in $(INSTALL_DIR)"; ok=false; \
	fi; \
	if [ -L "$(DIRENV_LIB)/dc.sh" ] && [ ! -e "$(DIRENV_LIB)/dc.sh" ]; then \
		echo "  ✗ direnv stdlib: $(DIRENV_LIB)/dc.sh is a dangling symlink → $$(readlink $(DIRENV_LIB)/dc.sh)"; \
		echo "    Run: make install-direnv-lib"; ok=false; \
	elif [ -f "$(DIRENV_LIB)/dc.sh" ] && [ -r "$(DIRENV_LIB)/dc.sh" ]; then \
		echo "  ✓ direnv stdlib: $(DIRENV_LIB)/dc.sh"; \
	else \
		echo "  ✗ direnv stdlib: $(DIRENV_LIB)/dc.sh missing"; ok=false; \
	fi; \
	if grep -qF 'dc-init' $(ZSHRC) 2>/dev/null; then \
		echo "  ✓ shell hook: present in $(ZSHRC)"; \
	else \
		echo "  ✗ shell hook: not found in $(ZSHRC)"; ok=false; \
	fi; \
	if command -v direnv >/dev/null 2>&1; then \
		echo "  ✓ direnv: $$(direnv version)"; \
	else \
		echo "  ✗ direnv: not installed (required)"; ok=false; \
	fi; \
	if command -v cargo >/dev/null 2>&1; then \
		echo "  ✓ cargo: $$(cargo --version | head -1) (build only)"; \
	else \
		echo "  · cargo: not installed (needed to compile from source)"; \
	fi; \
	$$ok && echo "==> All required components installed" || echo "==> Some components missing — run: make install"

doctor: check
	@echo ""
	@echo "==> Checking state directory"
	@if [ -d "$(STATE_DIR)" ]; then \
		count=$$(find $(STATE_DIR) -maxdepth 1 -type d | wc -l | tr -d ' '); \
		echo "  ✓ $(STATE_DIR) exists ($$((count - 1)) stores)"; \
	else \
		echo "  · $(STATE_DIR) does not exist yet (created on first dc_yaml call)"; \
	fi
	@echo ""
	@echo "==> Checking direnv stdlib is current"
	@if [ -L "$(DIRENV_LIB)/dc.sh" ]; then \
		echo "  · $(DIRENV_LIB)/dc.sh is a legacy symlink → $$(readlink $(DIRENV_LIB)/dc.sh)"; \
		echo "    Run: make install-direnv-lib  (installs a copy)"; \
	elif [ -f "$(DIRENV_LIB)/dc.sh" ]; then \
		if cmp -s "$(DIRENV_LIB)/dc.sh" "$(DC_HOME)/lib/direnv-stdlib.sh"; then \
			echo "  ✓ Installed copy matches lib/direnv-stdlib.sh"; \
		else \
			echo "  · Installed copy differs from lib/direnv-stdlib.sh"; \
			echo "    Run: make install-direnv-lib"; \
		fi; \
	fi
	@echo ""
	@echo "==> Checking Rust toolchain"
	@if command -v rustc >/dev/null 2>&1; then \
		echo "  ✓ rustc: $$(rustc --version)"; \
		echo "  ✓ target: $$(rustc -vV | grep host | cut -d' ' -f2)"; \
	else \
		echo "  ✗ rustc: not installed"; \
		echo "    Install: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"; \
	fi

# --- SDK orchestration ---

sdk-test:
	$(MAKE) -C sdk test

sdk-build:
	$(MAKE) -C sdk build

sdk-publish:
	$(MAKE) -C sdk publish

sdk-public: sdk-publish

sdk-clean:
	$(MAKE) -C sdk clean

# --- Clean ---

clean:
	@echo "==> Cleaning build artifacts"
	@if [ -f Cargo.toml ]; then \
		if command -v $(CARGO) >/dev/null 2>&1; then \
			$(CARGO) clean; \
		else \
			echo "==> Skipping clean: cargo not found."; \
		fi; \
	fi
	@rm -rf .test-tmp

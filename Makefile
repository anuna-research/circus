# circus — build harness (layout per ../elephant and ../zetl conventions)

PREFIX ?= $(HOME)/.local

# The controlled-language checker lives with the anuna-dev skill rather than in
# this repo. Every target that uses it degrades to a notice when it is absent,
# so a clone without the skill still builds and tests.
USDD_LINT ?= $(HOME)/Code/anuna-dev-skill/tools/usdd-lint.sh

.PHONY: all build test check lint clippy fmt fmt-fix mutants spec docs-lint \
        install uninstall dist clean doc doc-open help

all: check build

build: ## Release build
	cargo build --release

test: ## Run all tests (needs git, tmux, and withdone on PATH)
	cargo test

check: test lint ## Tests + lint

lint: ## rustfmt --check + clippy -D warnings
	cargo fmt --check
	cargo clippy --all-targets -- -D warnings

clippy:
	cargo clippy --all-targets -- -D warnings

fmt:
	cargo fmt --check

fmt-fix: ## Apply rustfmt
	cargo fmt

# Mutation testing on the pure core, which the Purity Boundary Map in
# SPEC-001-circus-agent-harness names as the critical module. The effectful
# shell is reachable only through the integration suite, which takes ~30s per
# run and makes whole-tree mutation impractical; git.rs and state.rs have unit
# tests, so they are included.
mutants: ## Mutation-test the pure core and the git/state modules
	cargo mutants --file 'src/core/*.rs' --file 'src/shell/git.rs' \
	              --file 'src/shell/state.rs' --test-tool cargo -- --lib

spec: ## Vault hygiene + controlled-language check on the specification
	@command -v zetl >/dev/null 2>&1 \
	  && zetl check --dead-links --fail-on error \
	  || echo "spec: zetl not installed, skipping the link check"
	@if [ -x "$(USDD_LINT)" ]; then \
	  sh "$(USDD_LINT)" specs/SPEC-001-circus-agent-harness.md; \
	else \
	  echo "spec: $(USDD_LINT) not found, skipping the prose check"; \
	fi

docs-lint: ## Controlled-language + mode check across the documentation
	@if [ ! -x "$(USDD_LINT)" ]; then \
	  echo "docs-lint: $(USDD_LINT) not found, skipping"; exit 0; \
	fi; \
	rc=0; \
	for f in $$(find docs drivers -name '*.md') README.md; do \
	  sh "$(USDD_LINT)" "$$f" >/dev/null 2>&1 || { echo "FAIL $$f"; rc=1; }; \
	  sh "$(USDD_LINT)" --doc "$$f" >/dev/null 2>&1 || true; \
	done; \
	[ $$rc -eq 0 ] && echo "docs-lint: clean"; exit $$rc

install: build ## Install to $(PREFIX)/bin
	install -d $(PREFIX)/bin
	install -m 755 target/release/circus $(PREFIX)/bin/circus

uninstall:
	rm -f $(PREFIX)/bin/circus

# Build the release binary and stage it as a distributable artifact for the
# host platform: dist/circus-<os>-<arch> plus its .sha256 checksum. The release
# pipeline (.woodpecker/release.yaml) produces all four platforms and uploads
# them to https://files.anuna.io/circus/; use this only to stage a single
# platform by hand (see scripts/install.sh).
dist: build ## Stage dist/circus-<os>-<arch> + .sha256 for the host platform
	@set -e; \
	os=$$(uname -s); arch=$$(uname -m); \
	case "$$os" in \
	  Darwin) os=darwin ;; \
	  Linux) os=linux ;; \
	  *) echo "dist: unsupported OS: $$os" >&2; exit 1 ;; \
	esac; \
	case "$$arch" in \
	  arm64|aarch64) arch=arm64 ;; \
	  x86_64|amd64) arch=x64 ;; \
	  *) echo "dist: unsupported architecture: $$arch" >&2; exit 1 ;; \
	esac; \
	artifact="circus-$$os-$$arch"; \
	mkdir -p dist; \
	cp target/release/circus "dist/$$artifact"; \
	if command -v sha256sum >/dev/null 2>&1; then \
	  (cd dist && sha256sum "$$artifact" > "$$artifact.sha256"); \
	else \
	  (cd dist && shasum -a 256 "$$artifact" > "$$artifact.sha256"); \
	fi; \
	echo ""; \
	echo "Staged dist/$$artifact and dist/$$artifact.sha256"

clean:
	cargo clean
	rm -rf dist mutants.out mutants.out.old

doc:
	cargo doc --no-deps

doc-open:
	cargo doc --no-deps --open

help: ## List targets
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  %-12s %s\n", $$1, $$2}'

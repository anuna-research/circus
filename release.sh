#!/bin/bash
# SPDX-License-Identifier: Apache-2.0
# Copyright (c) 2026 Anuna Research

set -e

# circus release script.
# Usage: ./release.sh [version]
# Example: ./release.sh 0.1.0
#
# Bumps the version, runs the gate, commits, tags, and pushes. Pushing the tag
# triggers .woodpecker/release.yaml, which cross-compiles the four prebuilt
# binaries and publishes them (plus scripts/install.sh and the example drivers)
# to Cloudflare R2, served at https://files.anuna.io/circus/.
#
# The gate here is the whole of it. Unlike the sibling repos, circus has no
# path dependencies to pin, so `--locked` alone fixes the tree and there is no
# lock-versus-pin reconciliation to do. What circus does have is a test suite
# that drives real Git, tmux, and withdone, and the release pipeline cannot run
# it: it cross-compiles from Linux containers to four targets, none of which
# can execute a tmux pane. So this script is the only place the suite runs
# before a tag exists, and it refuses to tag without it.

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info() { echo -e "${GREEN}[INFO]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }

VERSION="${1:-}"

if [[ -z "$VERSION" ]]; then
  CURRENT=$(grep '^version' Cargo.toml | head -1 | grep -o '"[^"]*"' | tr -d '"')
  echo "Current version: $CURRENT"
  read -r -p "Enter new version (without 'v' prefix): " VERSION
fi

[[ -z "$VERSION" ]] && error "Version is required"

if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$ ]]; then
  error "Invalid version format. Use semver: X.Y.Z or X.Y.Z-suffix"
fi

TAG="v$VERSION"

info "Preparing release $TAG"

if ! git diff --quiet || ! git diff --cached --quiet; then
  error "You have uncommitted changes. Commit or stash them first."
fi

if [[ "$(git rev-parse --abbrev-ref HEAD)" != "main" ]]; then
  error "Release from main; you are on $(git rev-parse --abbrev-ref HEAD)."
fi

if git rev-parse "$TAG" >/dev/null 2>&1; then
  error "Tag $TAG already exists"
fi

# The three programs circus composes. Without them the suite exits 127 fifteen
# times over, which is the suite reporting its environment correctly and not a
# reason to guess.
for prog in git tmux withdone; do
  command -v "$prog" >/dev/null 2>&1 \
    || error "\`$prog\` is not on PATH. The test suite drives it for real; install it and re-run."
done

info "Updating version in Cargo.toml..."
if [[ "$(uname)" == "Darwin" ]]; then
  sed -i '' "s/^version = \"[^\"]*\"/version = \"$VERSION\"/" Cargo.toml
else
  sed -i "s/^version = \"[^\"]*\"/version = \"$VERSION\"/" Cargo.toml
fi

info "Running tests (real git, tmux, and withdone; takes ~40s)..."
cargo test --quiet

info "Checking formatting..."
cargo fmt --check

info "Running clippy..."
cargo clippy --all-targets --quiet -- -D warnings

# CI builds with `--locked`. Proving the lock resolves now turns a pipeline
# failure four cross-compiles deep into a message before the tag exists.
info "Verifying Cargo.lock resolves under --locked..."
cargo metadata --locked --format-version 1 >/dev/null \
  || error "Cargo.lock is out of date. Run \`cargo build\` to refresh it, then re-run."

# The release pipeline reads these. A tag that publishes a broken artifact set
# is worse than one that never gets cut.
for required in scripts/install.sh .woodpecker/release.yaml; do
  [[ -f "$required" ]] || error "$required is missing; the release pipeline needs it."
done
[[ -n "$(echo drivers/circus-driver-*)" ]] || warn "No drivers found to publish."

# Specification hygiene, when the tooling is present. Not fatal: a clone
# without zetl can still cut a release.
if command -v zetl >/dev/null 2>&1; then
  info "Checking the specification vault..."
  zetl check --dead-links --fail-on error >/dev/null \
    || error "zetl reports dead links. Fix them or defer the release."
else
  warn "zetl not installed; skipping the specification link check."
fi

info "Committing version bump..."
# `cargo test` above refreshes circus's own version in Cargo.lock.
git add Cargo.toml Cargo.lock
git commit -m "chore: release $TAG"

info "Creating tag $TAG..."
git tag -a "$TAG" -m "Release $VERSION"

info "Pushing to origin..."
git push origin main
git push origin "$TAG"

echo ""
info "Release $TAG published!"
echo ""
echo "Woodpecker release pipeline triggered (.woodpecker/release.yaml)."
echo "When it finishes, the release will be available at:"
echo "  https://files.anuna.io/circus/            (latest)"
echo "  https://files.anuna.io/circus/$TAG/"
echo ""
echo "Install:  curl https://files.anuna.io/circus/install.sh | sh"
echo "Drivers:  https://files.anuna.io/circus/drivers/"

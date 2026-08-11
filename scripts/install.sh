#!/bin/sh
# SPDX-License-Identifier: Apache-2.0
# Copyright (c) 2026 Anuna Research
#
# circus installer.
#
# End users run:
#
#   curl https://files.anuna.io/circus/install.sh | sh
#
# Detects the platform, downloads the matching prebuilt binary from
# https://files.anuna.io/circus/, verifies its SHA-256 checksum, and installs
# it to $CIRCUS_INSTALL_DIR (default: ~/.local/bin). No Rust toolchain
# required. curl downloads do not carry macOS's com.apple.quarantine
# attribute, so notarisation is optional.
#
# Supported artifacts:
#   circus-darwin-arm64  circus-darwin-x64  circus-linux-x64  circus-linux-arm64
#
# Also installs circus-driver-codex, circus-driver-claude, and
# circus-driver-opencode — the three example drivers (of drivers/README.md's
# four) that run non-interactively and need no extra setup beyond an auth
# login (`codex login` / an Anthropic API key / an opencode provider). This
# does not put provider knowledge in the circus binary itself
# (SPEC-001-circus-agent-harness#REQ-007.b is about the binary, not the
# installer); it just means `circus spawn ... -- circus-driver-codex` works
# right after install instead of requiring a separate clone-and-copy step.
# A failure fetching a driver is a warning, not a fatal error — the binary is
# the artifact this script exists to install. Set
# CIRCUS_INSTALL_DRIVERS=0 to skip them entirely.
#
# circus composes Git, tmux, and withdone as external programs rather than
# reimplementing them, so it needs all three at run time. This script does not
# install them — a harness that silently pulls in three dependencies is not
# composing them — but it does check and say what is missing.
#
# Publishing (operator):
#   Tagging a release with ./release.sh triggers .forgejo/workflows/release.yaml,
#   which cross-compiles the four binaries and uploads them (plus this script
#   as install.sh) to https://files.anuna.io/circus/. To stage a
#   single-platform artifact by hand, run `make dist`.
#
# Environment overrides:
#   CIRCUS_BASE_URL        - artifact base URL (default: https://files.anuna.io/circus)
#   CIRCUS_INSTALL_DIR     - install directory (default: ~/.local/bin)
#   CIRCUS_INSTALL_DRIVERS - install the example drivers too (default: 1)
#
# Testing: `INSTALL_SH_TEST=1 . scripts/install.sh` sources the functions
# without running the installation.

set -eu

BASE_URL="${CIRCUS_BASE_URL:-https://files.anuna.io/circus}"
INSTALL_DIR="${CIRCUS_INSTALL_DIR:-$HOME/.local/bin}"
INSTALL_DRIVERS="${CIRCUS_INSTALL_DRIVERS:-1}"

info() { printf '%s\n' "$*"; }
warn() { printf 'warning: %s\n' "$*" >&2; }
die()  { printf 'error: %s\n' "$*" >&2; exit 1; }

# Map uname output onto the artifact names the release pipeline publishes.
detect_platform() {
    _os=$(uname -s)
    _arch=$(uname -m)
    case "$_os" in
        Darwin) _os=darwin ;;
        Linux)  _os=linux ;;
        *) die "unsupported operating system: $_os. Build from source with \`make install\`." ;;
    esac
    case "$_arch" in
        arm64|aarch64) _arch=arm64 ;;
        x86_64|amd64)  _arch=x64 ;;
        *) die "unsupported architecture: $_arch. Build from source with \`make install\`." ;;
    esac
    printf 'circus-%s-%s' "$_os" "$_arch"
}

fetch() {
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$1" -o "$2"
    elif command -v wget >/dev/null 2>&1; then
        wget -qO "$2" "$1"
    else
        die "neither curl nor wget is available"
    fi
}

# Verify the download against the published checksum. A mismatch is fatal:
# a partial download and a tampered one look the same from here.
verify_checksum() {
    _file="$1"
    _sums="$2"
    _expected=$(awk '{print $1}' < "$_sums")
    [ -n "$_expected" ] || die "the published checksum file is empty"

    if command -v sha256sum >/dev/null 2>&1; then
        _actual=$(sha256sum "$_file" | awk '{print $1}')
    elif command -v shasum >/dev/null 2>&1; then
        _actual=$(shasum -a 256 "$_file" | awk '{print $1}')
    else
        warn "no sha256sum or shasum found; skipping checksum verification"
        return 0
    fi

    [ "$_actual" = "$_expected" ] || die "checksum mismatch.
  expected $_expected
  got      $_actual
Delete the download and try again; if it persists, the artifact is wrong."
}

# The three drivers that work with nothing more than an auth login: no tmux
# TUI handling (circus-driver-claude-tui is experimental and broken — see
# drivers/README.md), no provider API key juggling beyond what the agent CLI
# itself already asks for. circus-driver-claude-tui stays an opt-in fetch from
# $BASE_URL/drivers/ for anyone who wants it.
install_drivers() {
    [ "$INSTALL_DRIVERS" = "1" ] || return 0

    for _driver in circus-driver-codex circus-driver-claude circus-driver-opencode; do
        _dtmp=$(mktemp -d)
        if fetch "$BASE_URL/drivers/$_driver" "$_dtmp/$_driver" \
            && fetch "$BASE_URL/drivers/$_driver.sha256" "$_dtmp/$_driver.sha256"; then
            # verify_checksum calls die() on mismatch, and die() calls exit —
            # which would otherwise end the whole script, not just this
            # optional step. A subshell confines that exit to the subshell.
            if ( verify_checksum "$_dtmp/$_driver" "$_dtmp/$_driver.sha256" ) 2>/dev/null; then
                chmod 755 "$_dtmp/$_driver"
                mv "$_dtmp/$_driver" "$INSTALL_DIR/$_driver"
                info "Installed $INSTALL_DIR/$_driver"
            else
                warn "checksum mismatch for $_driver; not installing it. Fetch it by hand from $BASE_URL/drivers/"
            fi
        else
            warn "could not fetch $_driver; skipping. Fetch it by hand from $BASE_URL/drivers/"
        fi
        rm -rf "$_dtmp"
    done
}

# circus invokes these three and never substitutes an internal implementation,
# so an absent one is a run-time failure with exit 127 rather than a silent
# fallback. Saying so now costs nothing.
check_companions() {
    _missing=""
    for _prog in git tmux withdone; do
        command -v "$_prog" >/dev/null 2>&1 || _missing="$_missing $_prog"
    done
    [ -n "$_missing" ] || return 0

    warn "circus needs these on PATH and they are not installed:$_missing"
    case "$_missing" in
        *withdone*)
            info "  withdone:  curl -fsSL https://git.anuna.io/anuna-research/withdone/raw/branch/main/withdone \\"
            info "               -o ~/.local/bin/withdone && chmod +x ~/.local/bin/withdone" ;;
    esac
    case "$_missing" in
        *tmux*)
            info "  tmux:      brew install tmux   (or your package manager)" ;;
    esac
    case "$_missing" in
        *git*)
            info "  git:       install Git for your platform" ;;
    esac
}

main() {
    artifact=$(detect_platform)
    info "Installing $artifact from $BASE_URL"

    tmp=$(mktemp -d)
    # shellcheck disable=SC2064  # expand $tmp now, so the trap works after unset
    trap "rm -rf '$tmp'" EXIT INT TERM

    fetch "$BASE_URL/$artifact"        "$tmp/circus" \
        || die "download failed. Is $BASE_URL/$artifact published?"
    fetch "$BASE_URL/$artifact.sha256" "$tmp/circus.sha256" \
        || die "could not download the checksum for $artifact"

    verify_checksum "$tmp/circus" "$tmp/circus.sha256"

    mkdir -p "$INSTALL_DIR"
    chmod 755 "$tmp/circus"
    mv "$tmp/circus" "$INSTALL_DIR/circus"

    info "Installed $INSTALL_DIR/circus"

    install_drivers

    case ":$PATH:" in
        *":$INSTALL_DIR:"*) ;;
        *) warn "$INSTALL_DIR is not on your PATH. Add it:"
           info "  export PATH=\"$INSTALL_DIR:\$PATH\"" ;;
    esac

    check_companions

    info ""
    info "Start here:  circus --help"
    if [ "$INSTALL_DRIVERS" = "1" ]; then
        info "Drivers:     circus-driver-codex, circus-driver-claude, and circus-driver-opencode installed."
        info "             More at $BASE_URL/drivers/ (circus-driver-claude-tui)"
    else
        info "Drivers:     $BASE_URL/drivers/"
    fi
    info "Docs:        https://git.anuna.io/anuna-research/circus"
}

# Sourcing with INSTALL_SH_TEST=1 exposes the functions without installing.
[ "${INSTALL_SH_TEST:-}" = "1" ] || main "$@"

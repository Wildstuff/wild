#!/bin/sh
# Cross-platform installer for `wild`.
#
# Usage:
#
#   curl -fsSL https://raw.githubusercontent.com/wildstuff/wild/main/install.sh | sh
#
# (or once the redirect is wired up: `curl -fsSL https://wildstuff.com/install | sh`).
#
# Pulls the prebuilt host binary from the GitHub Release matching
# the user's OS + arch, verifies its sha256, and installs to
# $WILD_INSTALL_DIR/wild (default: $HOME/.wild/bin/wild).
#
# Knobs (env vars):
#
#   WILD_VERSION       — pin to a specific tag (e.g. v0.1.2). Default: latest release.
#   WILD_INSTALL_DIR   — install location. Default: $HOME/.wild/bin.
#   WILD_NO_MODIFY_PATH — set to 1 to skip the PATH-hint output.
#
# Supported targets (matches .github/workflows/release.yml's build matrix):
#
#   - macOS arm64  → aarch64-apple-darwin
#   - macOS x86_64 → x86_64-apple-darwin
#   - Linux x86_64 → x86_64-unknown-linux-gnu
#
# Linux aarch64 isn't published yet; the script exits with a
# build-from-source hint there. Windows isn't supported — use WSL.
#
# Requires: curl OR wget, tar, and one of `shasum` / `sha256sum`.

# ADR-0225 D2 — releases live in the PUBLIC distribution repo, not in the
# development repo this file is authored in. GitHub Release assets inherit
# repo visibility, so an anonymous `releases/latest` against a private repo
# 404s; that is why the installer this repo shipped never worked. `wildstuff/wild`
# is generated from here by `xtask public-sync`, and this script is synced
# into it, so the reference below is to the repo a user actually reaches.
#
# Note that GHCR packages are NOT affected by that rule: their namespace is
# org-scoped with a per-package visibility switch, so no `oci://ghcr.io/wildstuff/…`
# reference anywhere changes.

set -eu

GITHUB_REPO="wildstuff/wild"
GITHUB_API="https://api.github.com/repos/${GITHUB_REPO}"
RELEASE_BASE="https://github.com/${GITHUB_REPO}/releases/download"

# ── helpers ──────────────────────────────────────────────────────────

err() {
    printf 'install: %s\n' "$*" >&2
    exit 1
}

info() {
    printf '%s\n' "$*"
}

# Pick curl or wget. We need one to fetch the API + the tarball.
have() { command -v "$1" >/dev/null 2>&1; }

if have curl; then
    fetch() { curl -fsSL "$1"; }
    fetch_to() { curl -fsSL -o "$2" "$1"; }
elif have wget; then
    fetch() { wget -qO- "$1"; }
    fetch_to() { wget -qO "$2" "$1"; }
else
    err "neither curl nor wget found; install one and re-run"
fi

# Pick the right sha256 tool. macOS ships shasum; Linux usually
# sha256sum. BusyBox's sha256sum also works.
if have sha256sum; then
    sha_check() { sha256sum -c "$1" >/dev/null; }
elif have shasum; then
    sha_check() { shasum -a 256 -c "$1" >/dev/null; }
else
    err "neither sha256sum nor shasum found; install coreutils or use a system with shasum"
fi

# ── OS / arch detection ──────────────────────────────────────────────

detect_target() {
    os="$(uname -s)"
    arch="$(uname -m)"
    case "$os" in
        Darwin)
            case "$arch" in
                arm64|aarch64) target="aarch64-apple-darwin" ;;
                x86_64)        target="x86_64-apple-darwin" ;;
                *) err "unsupported macOS arch: $arch" ;;
            esac
            ;;
        Linux)
            case "$arch" in
                x86_64|amd64) target="x86_64-unknown-linux-gnu" ;;
                aarch64|arm64)
                    err "Linux aarch64 isn't published yet. Track https://github.com/${GITHUB_REPO}/issues for the aarch64-linux build, or build from source: 'git clone https://github.com/${GITHUB_REPO} && cargo build -p wild-frontend -p wild-daemon --release'."
                    ;;
                *) err "unsupported Linux arch: $arch" ;;
            esac
            ;;
        MINGW*|MSYS*|CYGWIN*|Windows_NT)
            err "Windows isn't supported. Use Windows Subsystem for Linux (WSL) and re-run this installer from inside the WSL shell, or build from source: 'git clone https://github.com/${GITHUB_REPO} && cargo build -p wild-frontend -p wild-daemon --release'."
            ;;
        *)
            err "unsupported OS: $os"
            ;;
    esac
    printf '%s' "$target"
}

# ── version resolution ───────────────────────────────────────────────

resolve_version() {
    if [ -n "${WILD_VERSION-}" ]; then
        # Accept both `0.1.2` and `v0.1.2`; normalise to the leading-v
        # tag form the release pipeline produces.
        case "$WILD_VERSION" in
            v*) printf '%s' "$WILD_VERSION" ;;
            *)  printf 'v%s' "$WILD_VERSION" ;;
        esac
        return
    fi
    # GitHub's `releases/latest` API returns the latest non-draft
    # release. Parse `tag_name` with sed — JSON parser would need
    # jq, which isn't installed everywhere. A 404 here means the
    # repo has no releases yet; we surface that explicitly so the
    # operator gets a useful hint instead of a "couldn't reach"
    # network-style error.
    if ! body="$(fetch "${GITHUB_API}/releases/latest" 2>/dev/null)"; then
        err "no published releases found for ${GITHUB_REPO} (API returned non-200). Pin a specific tag with WILD_VERSION=v0.1.2, or build from source: 'git clone https://github.com/${GITHUB_REPO} && cargo build -p wild-frontend -p wild-daemon --release'."
    fi
    tag="$(printf '%s' "$body" \
        | grep '"tag_name":' \
        | head -n 1 \
        | sed -E 's/.*"tag_name"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/')"
    [ -n "$tag" ] || err "couldn't parse tag_name from latest release JSON (got: $(printf '%s' "$body" | head -c 200))"
    printf '%s' "$tag"
}

# ── main ─────────────────────────────────────────────────────────────

main() {
    target="$(detect_target)"
    tag="$(resolve_version)"
    version="${tag#v}"

    install_dir="${WILD_INSTALL_DIR:-$HOME/.wild/bin}"
    asset="wild-${version}-${target}.tar.gz"
    asset_url="${RELEASE_BASE}/${tag}/${asset}"
    sha_url="${asset_url}.sha256"

    info "Installing wild ${tag} for ${target}"
    info "  source:  ${asset_url}"
    info "  destdir: ${install_dir}"

    # Stage in a tmp dir we own, then atomically move the binary
    # into place. Trap cleans up on any exit path.
    tmpdir="$(mktemp -d 2>/dev/null || mktemp -d -t wild-install)"
    trap 'rm -rf "$tmpdir"' EXIT INT HUP TERM

    info ""
    info "→ downloading tarball + sha256 sidecar"
    fetch_to "$asset_url" "$tmpdir/$asset" \
        || err "couldn't download $asset_url"
    fetch_to "$sha_url" "$tmpdir/$asset.sha256" \
        || err "couldn't download $sha_url"

    info "→ verifying sha256"
    # The .sha256 sidecar is `<hex>  <filename>` against the bare
    # `wild-<v>-<target>.tar.gz` filename, so cd into the staging
    # dir for the check.
    ( cd "$tmpdir" && sha_check "$asset.sha256" ) \
        || err "sha256 mismatch for $asset — abort. Re-run later, or report at https://github.com/${GITHUB_REPO}/issues."

    info "→ extracting"
    tar -xzf "$tmpdir/$asset" -C "$tmpdir"
    extracted="$tmpdir/wild-${version}-${target}/wild"
    [ -x "$extracted" ] || err "binary not found at expected path inside tarball: $extracted"

    info "→ installing binary"
    mkdir -p "$install_dir"
    # mv across filesystems can fall back to cp+rm — both are fine.
    mv -f "$extracted" "$install_dir/wild"
    chmod +x "$install_dir/wild"

    info ""
    info "✓ wild ${tag} installed → ${install_dir}/wild"

    # PATH hint — only if the install dir isn't already on PATH.
    if [ "${WILD_NO_MODIFY_PATH:-0}" != "1" ]; then
        case ":$PATH:" in
            *":$install_dir:"*) ;;
            *)
                info ""
                info "Add ${install_dir} to your PATH:"
                info ""
                info "    echo 'export PATH=\"${install_dir}:\$PATH\"' >> ~/.bashrc"
                info "    echo 'export PATH=\"${install_dir}:\$PATH\"' >> ~/.zshrc"
                info ""
                ;;
        esac
    fi

    info "Optional runtime add-ons (not pulled by this installer):"
    info "  • nats-server  — embedded host can supervise its own;"
    info "                   install separately if you prefer to point"
    info "                   wild at an existing NATS via WILD_NATS_URL."
    info "  • docker       — only needed for the Forge component build sandbox."
    info "  • claude CLI   — for the anthropic-cli LLM adapter."
    info ""
    info "Quick start:"
    info "  wild up                    # boot the embedded host"
    info "  wild doctor                # health snapshot"
    info ""
    info "Docs: https://github.com/${GITHUB_REPO}"
}

main "$@"

#!/bin/sh
# gitlab-cli installer
# usage:
#   curl -sSL https://raw.githubusercontent.com/zhiyue/gitlab-cli/main/install.sh | sh
#   ./install.sh                       # install latest
#   ./install.sh -v v0.1.0             # specific version
#   ./install.sh -d /usr/local/bin     # specific dir
#   ./install.sh -b https://internal.example.com/gitlab-cli   # custom base URL (internal mirror)
set -eu

REPO_DEFAULT="zhiyue/gitlab-cli"
REPO="${GITLAB_CLI_REPO:-$REPO_DEFAULT}"
BASE_URL_DEFAULT="https://github.com/${REPO}/releases"
BASE_URL="${GITLAB_CLI_DOWNLOAD_URL:-$BASE_URL_DEFAULT}"
VERSION=""
INSTALL_DIR=""

while [ $# -gt 0 ]; do
    case "$1" in
        -v|--version) VERSION="$2"; shift 2 ;;
        -d|--dir) INSTALL_DIR="$2"; shift 2 ;;
        -b|--base-url) BASE_URL="$2"; shift 2 ;;
        -h|--help)
            echo "usage: install.sh [-v VERSION] [-d DIR] [-b BASE_URL]"
            echo "env: GITLAB_CLI_REPO, GITLAB_CLI_DOWNLOAD_URL"
            exit 0 ;;
        *) echo "unknown flag: $1" >&2; exit 2 ;;
    esac
done

# Detect target triple
detect_target() {
    os="$(uname -s | tr '[:upper:]' '[:lower:]')"
    arch="$(uname -m)"
    case "$arch" in
        x86_64|amd64) arch="x86_64" ;;
        arm64|aarch64) arch="aarch64" ;;
        *) echo "unsupported arch: $arch" >&2; exit 1 ;;
    esac
    case "$os" in
        darwin) echo "${arch}-apple-darwin" ;;
        linux)
            # Prefer gnu over musl when both available; here we always use gnu since
            # release.yml ships both — gnu is the natural default.
            echo "${arch}-unknown-linux-gnu" ;;
        *) echo "unsupported OS: $os (use Homebrew on macOS or download Windows zip from Releases)" >&2; exit 1 ;;
    esac
}

# Resolve latest version if not set
resolve_version() {
    if [ -n "$VERSION" ]; then
        printf '%s' "$VERSION"
        return
    fi
    api="https://api.github.com/repos/${REPO}/releases/latest"
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$api" | sed -n 's/.*"tag_name":[[:space:]]*"\(v[^"]*\)".*/\1/p' | head -1
    else
        wget -qO- "$api" | sed -n 's/.*"tag_name":[[:space:]]*"\(v[^"]*\)".*/\1/p' | head -1
    fi
}

# Pick install dir
default_install_dir() {
    if [ -w "/usr/local/bin" ] 2>/dev/null; then
        echo "/usr/local/bin"
    elif [ -d "$HOME/.local/bin" ]; then
        echo "$HOME/.local/bin"
    else
        mkdir -p "$HOME/.local/bin"
        echo "$HOME/.local/bin"
    fi
}

TARGET="$(detect_target)"
TAG="$(resolve_version)"
if [ -z "$TAG" ]; then
    echo "error: could not resolve latest version from GitHub API" >&2
    exit 1
fi
[ -z "$INSTALL_DIR" ] && INSTALL_DIR="$(default_install_dir)"

ARCHIVE="gitlab-cli-${TAG}-${TARGET}.tar.gz"
SUM="gitlab-cli-${TAG}-${TARGET}.tar.gz.sha256"
URL="${BASE_URL}/download/${TAG}/${ARCHIVE}"
SUM_URL="${BASE_URL}/download/${TAG}/${SUM}"

echo "==> Installing gitlab-cli ${TAG} (${TARGET}) → ${INSTALL_DIR}"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$URL"     -o "$TMP/$ARCHIVE"
    curl -fsSL "$SUM_URL" -o "$TMP/$SUM" || true
else
    wget -qO "$TMP/$ARCHIVE" "$URL"
    wget -qO "$TMP/$SUM" "$SUM_URL" || true
fi

# Verify checksum if available
if [ -s "$TMP/$SUM" ]; then
    expected="$(awk '{print $1}' "$TMP/$SUM")"
    if command -v sha256sum >/dev/null 2>&1; then
        actual="$(sha256sum "$TMP/$ARCHIVE" | awk '{print $1}')"
    else
        actual="$(shasum -a 256 "$TMP/$ARCHIVE" | awk '{print $1}')"
    fi
    if [ "$expected" != "$actual" ]; then
        echo "checksum mismatch: expected $expected got $actual" >&2
        exit 1
    fi
    echo "==> sha256 verified"
fi

tar -xzf "$TMP/$ARCHIVE" -C "$TMP"
install -m 0755 "$TMP/gitlab" "$INSTALL_DIR/gitlab"

echo "==> Installed: $INSTALL_DIR/gitlab"
"$INSTALL_DIR/gitlab" --version || true
echo
echo "Next: configure your token"
echo "  gitlab config set-token --host https://gitlab.example.com --token glpat-xxxxx"
echo "  gitlab version"

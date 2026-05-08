#!/usr/bin/env sh
set -eu
export LC_ALL=C   # locale-stable sort/awk/grep across macOS / Linux

usage() {
  cat <<'USAGE'
Install Convergio prebuilt binaries from GitHub Releases.

Defaults:
  repo: Roberdan/convergio
  install dir: ~/.local/bin
  version: latest GitHub release

One-liner:
  curl -fsSL https://raw.githubusercontent.com/Roberdan/convergio/main/scripts/install.sh | sh

Options:
  --repo <owner/repo>       Override GitHub repo (default: Roberdan/convergio)
  --tag <tag>               Release tag or version to install (default: latest)
                            Examples: latest, X.Y.Z, vX.Y.Z, convergio-vX.Y.Z
  --version <tag>           Alias for --tag
  --dir <path>              Install directory (default: ~/.local/bin)
  --prefix <path>           Install prefix (installs to <prefix>/bin)
  -h, --help                Show help

Environment variables (equivalent):
  CONVERGIO_REPO
  CONVERGIO_TAG / CONVERGIO_VERSION
  CONVERGIO_INSTALL_DIR / PREFIX

Notes:
  - Supported prebuilt platforms: macos-arm64, linux-x86_64.
  - For other platforms, build from source (see README).
USAGE
}

fail() {
  echo "error: $*" >&2
  exit 1
}

need() {
  command -v "$1" >/dev/null 2>&1 || fail "missing required command: $1"
}

fetch_to() {
  url="$1"
  out="$2"

  if command -v curl >/dev/null 2>&1; then
    if curl -fsSL "$url" -o "$out"; then
      return 0
    fi
    return 1
  fi

  if command -v wget >/dev/null 2>&1; then
    if wget -qO "$out" "$url"; then
      return 0
    fi
    return 1
  fi

  fail "need curl or wget to download release assets"
}

sha256_file() {
  file="$1"

  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$file" | awk '{print $1}'
    return
  fi

  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$file" | awk '{print $1}'
    return
  fi

  if command -v openssl >/dev/null 2>&1; then
    openssl dgst -sha256 "$file" | awk '{print $NF}'
    return
  fi

  fail "need shasum, sha256sum, or openssl to verify SHA-256"
}

repo="${CONVERGIO_REPO:-Roberdan/convergio}"
tag="${CONVERGIO_TAG:-${CONVERGIO_VERSION:-latest}}"
install_dir="${CONVERGIO_INSTALL_DIR:-}"
prefix="${PREFIX:-}"

while [ $# -gt 0 ]; do
  case "$1" in
    --repo)
      [ $# -ge 2 ] || fail "--repo requires a value"
      repo="$2"
      shift 2
      ;;
    --tag|--version)
      [ $# -ge 2 ] || fail "$1 requires a value"
      tag="$2"
      shift 2
      ;;
    --dir)
      [ $# -ge 2 ] || fail "--dir requires a value"
      install_dir="$2"
      shift 2
      ;;
    --prefix)
      [ $# -ge 2 ] || fail "--prefix requires a value"
      prefix="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      fail "unknown argument: $1"
      ;;
  esac
done

if [ -z "$install_dir" ]; then
  if [ -n "$prefix" ]; then
    install_dir="$prefix/bin"
  else
    install_dir="$HOME/.local/bin"
  fi
fi

os=$(uname -s)
arch=$(uname -m)
platform=""
case "${os}_${arch}" in
  Darwin_arm64|Darwin_aarch64)
    platform="macos-arm64"
    ;;
  Linux_x86_64|Linux_amd64)
    platform="linux-x86_64"
    ;;
  *)
    fail "unsupported platform: os=$os arch=$arch (supported: macOS arm64, Linux x86_64)"
    ;;
esac

asset="convergio-${platform}.tar.gz"
checksums="convergio-${platform}.SHA256SUMS"

need tar

workdir=$(mktemp -d 2>/dev/null || mktemp -d -t convergio-install)
trap 'rm -rf "$workdir"' EXIT INT TERM

tar_path="$workdir/$asset"
sums_path="$workdir/$checksums"

candidate_tags=""
case "$tag" in
  latest)
    candidate_tags="latest"
    ;;
  *)
    candidate_tags="$tag"
    case "$tag" in
      convergio-*|*-v*)
        ;;
      v*)
        candidate_tags="$candidate_tags convergio-$tag"
        ;;
      *)
        candidate_tags="$candidate_tags convergio-v$tag v$tag"
        ;;
    esac
    ;;
esac

base_url=""
resolved_tag=""
for cand in $candidate_tags; do
  if [ "$cand" = "latest" ]; then
    url="https://github.com/$repo/releases/latest/download"
  else
    url="https://github.com/$repo/releases/download/$cand"
  fi

  if fetch_to "$url/$checksums" "$sums_path"; then
    base_url="$url"
    resolved_tag="$cand"
    break
  fi

done

[ -n "$base_url" ] || fail "could not find release checksums for tag '$tag' (tried: $candidate_tags)"

fetch_to "$base_url/$asset" "$tar_path" || fail "failed to download $asset from $base_url"

expected=$(awk -v f="$asset" '$(NF)==f {print $1}' "$sums_path" | head -n 1)
[ -n "$expected" ] || fail "checksum file did not include entry for $asset"

actual=$(sha256_file "$tar_path")
[ "$actual" = "$expected" ] || fail "checksum mismatch for $asset (expected $expected, got $actual)"

mkdir -p "$install_dir"

tar -C "$workdir" -xzf "$tar_path"
pkg_dir="$workdir/convergio-${platform}"
[ -d "$pkg_dir/bin" ] || fail "unexpected archive layout (missing $pkg_dir/bin)"

copy_bin() {
  name="$1"
  src="$pkg_dir/bin/$name"
  dst="$install_dir/$name"

  [ -f "$src" ] || fail "missing $name in archive"

  if command -v install >/dev/null 2>&1; then
    install -m 0755 "$src" "$dst"
  else
    cp "$src" "$dst"
    chmod 0755 "$dst"
  fi
}

copy_bin convergio
copy_bin cvg
copy_bin convergio-mcp

warn_shadow() {
  name="$1"
  expected="$install_dir/$name"
  actual_path=$(command -v "$name" 2>/dev/null || true)
  if [ -n "$actual_path" ] && [ "$actual_path" != "$expected" ]; then
    cat >&2 <<WARN
WARN: '$name' on PATH is '$actual_path', but this installer wrote '$expected'.
      Fix by putting '$install_dir' earlier in PATH.
WARN
  fi
}

warn_shadow convergio
warn_shadow cvg
warn_shadow convergio-mcp

cat <<MSG
Installed Convergio binaries to:
  $install_dir

Next:
  cvg setup
  convergio start

If the commands are not found, ensure your PATH includes:
  $install_dir
MSG

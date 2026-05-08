#!/usr/bin/env sh
set -eu
export LC_ALL=C   # locale-stable sort/awk/grep across macOS / Linux CI (T1.19 / F27)

repo_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_dir"

cargo build --release --workspace

os=$(uname -s)
arch=$(uname -m)
case "$os" in
  Darwin) os=macos ;;
  Linux) os=linux ;;
  *) echo "error: unsupported OS: $os" >&2; exit 1 ;;
esac

case "$arch" in
  arm64|aarch64) arch=arm64 ;;
  x86_64|amd64) arch=x86_64 ;;
  *) echo "error: unsupported arch: $arch" >&2; exit 1 ;;
esac

name="convergio-$os-$arch"
rm -rf "dist/$name"
mkdir -p "dist/$name/bin"
cp target/release/convergio "dist/$name/bin/"
cp target/release/cvg "dist/$name/bin/"
cp target/release/convergio-mcp "dist/$name/bin/"
cp README.md LICENSE "dist/$name/"

tarball="dist/$name.tar.gz"
tar -C dist -czf "$tarball" "$name"

# Best-effort local checksum file (CI also publishes SBOM + checksums).
# Match the CI/release format: checksum lines end with the *basename*, so the
# installer can look up `convergio-<platform>.tar.gz` in `<platform>.SHA256SUMS`.
if command -v shasum >/dev/null 2>&1; then
  (cd dist && shasum -a 256 "$name.tar.gz" > "$name.SHA256SUMS")
elif command -v sha256sum >/dev/null 2>&1; then
  (cd dist && sha256sum "$name.tar.gz" > "$name.SHA256SUMS")
fi

echo "$tarball"

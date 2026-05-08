#!/usr/bin/env sh
set -eu
export LC_ALL=C   # locale-stable sort/awk/grep across macOS / Linux CI (T1.19 / F27)

repo_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_dir"

identity="${APPLE_SIGNING_IDENTITY:-}"
artifact_dir="${1:-dist/convergio-macos-arm64}"

if [ -z "$identity" ]; then
  identity=$(security find-identity -v -p codesigning \
    | awk -F '"' '/Developer ID Application/ {print $2; exit}')
fi

if [ -z "$identity" ]; then
  echo "No Developer ID Application identity found." >&2
  echo "Set APPLE_SIGNING_IDENTITY or install a Developer ID Application certificate." >&2
  exit 1
fi

for bin in "$artifact_dir/bin/convergio" "$artifact_dir/bin/cvg" "$artifact_dir/bin/convergio-mcp"; do
  codesign --force --timestamp --options runtime --sign "$identity" "$bin"
  codesign --verify --strict --verbose=2 "$bin"
done

zip_path="$artifact_dir-signed.zip"
rm -f "$zip_path"
ditto -c -k --keepParent "$artifact_dir" "$zip_path"
shasum -a 256 "$zip_path" > "$zip_path.sha256"

if [ -n "${APPLE_NOTARY_PROFILE:-}" ]; then
  xcrun notarytool submit "$zip_path" --keychain-profile "$APPLE_NOTARY_PROFILE" --wait
elif [ -n "${APPLE_API_KEY_PATH:-}" ] && [ -n "${APPLE_API_KEY_ID:-}" ] && [ -n "${APPLE_API_ISSUER_ID:-}" ]; then
  xcrun notarytool submit "$zip_path" \
    --key "$APPLE_API_KEY_PATH" \
    --key-id "$APPLE_API_KEY_ID" \
    --issuer "$APPLE_API_ISSUER_ID" \
    --wait
else
  echo "Signed $zip_path"
  echo "Notarization skipped: set APPLE_NOTARY_PROFILE or APPLE_API_KEY_PATH/ID/ISSUER."
fi

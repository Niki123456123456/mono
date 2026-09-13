#!/bin/bash
set -euo pipefail

# Run from the repository root after building the requested target.
target="${1:?Usage: package-macos.sh <rust-target> <architecture-label>}"
arch="${2:?Usage: package-macos.sh <rust-target> <architecture-label>}"
binary="target/$target/release/deployboard"
test -x "$binary"
staging="$(mktemp -d)"
trap 'rm -rf "$staging"' EXIT
app="$staging/deployboard.app"
mkdir -p "$app/Contents/MacOS" dist
cp "$binary" "$app/Contents/MacOS/deployboard"

python3 - "$app/Contents/Info.plist" <<'PY'
import plistlib
import sys
import tomllib

with open("crates/deployboard/Cargo.toml", "rb") as manifest:
    version = tomllib.load(manifest)["package"]["version"]
with open(sys.argv[1], "wb") as plist:
    plistlib.dump({
        "CFBundleName": "deployboard",
        "CFBundleDisplayName": "Deployboard",
        "CFBundleIdentifier": "com.niklasnordkamp.deployboard",
        "CFBundleExecutable": "deployboard",
        "CFBundlePackageType": "APPL",
        "CFBundleInfoDictionaryVersion": "6.0",
        "CFBundleShortVersionString": version,
        "CFBundleVersion": version,
        "LSMinimumSystemVersion": "13.0",
        "NSHighResolutionCapable": True,
    }, plist)
PY

plutil -lint "$app/Contents/Info.plist"
codesign --force --sign - "$app"
codesign --verify --strict --verbose=2 "$app"
ln -s /Applications "$staging/Applications"
hdiutil create -volname "Deployboard" -srcfolder "$staging" -ov -format UDZO \
  "dist/deployboard-macos-$arch.dmg"
hdiutil verify "dist/deployboard-macos-$arch.dmg"

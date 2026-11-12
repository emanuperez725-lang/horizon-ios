#!/usr/bin/env bash
# Horizon.ipa from a built iOS client, on a Mac: packaging/ios/package.sh BINARY [OUT_DIR].
# Signed ad hoc, for sideloading. It holds no game files: the player copies the resources
# prepared on a computer to Documents/horizon/resources (packaging/ios/README.md).
set -euo pipefail
binary=$1
out=${2:-dist}
here=$(cd "$(dirname "$0")" && pwd)
app="$out/Payload/Horizon.app"

rm -rf "$out/Payload" "$out/Horizon.ipa"
mkdir -p "$app"
cp "$binary" "$app/horizon-client"
cp "$here/Info.plist" "$here/../../LICENSE" "$app/"
for scale in 2 3; do
  side=$((60 * scale))
  sips -z "$side" "$side" "$here/../icons/hicolor/512x512/apps/horizon.png" \
    --out "$app/AppIcon60x60@${scale}x.png" >/dev/null
done
codesign --force --sign - "$app"
(cd "$out" && zip -qry Horizon.ipa Payload)
ls -lh "$out/Horizon.ipa"

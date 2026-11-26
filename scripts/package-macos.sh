#!/bin/sh
# Builds dist/Shiftella.app and dist/Shiftella-macos.zip.
#
#   ./scripts/package-macos.sh
#
# The game loads resources by relative paths (assets/...), so the bundle's
# executable is a tiny launcher script: it cds into Contents/Resources and
# execs the real binary.
#
# Note: generated audio (assets/audio/music, assets/audio/sfx) is not in git
# (ElevenLabs licence); it is bundled only if present locally. The CC0 sounds
# (assets/audio/cc0, credits in assets/audio/CREDITS.md) are always bundled.
set -eu

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export PATH="/opt/homebrew/opt/rustup/bin:$HOME/.cargo/bin:$PATH"

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
APP="dist/Shiftella.app"

echo "→ release build v$VERSION"
cargo build --release --bin shiftella-game

# Make sure the generated translations are up to date
cargo run --release --quiet --example content_i18n_build >/dev/null

echo "→ bundle"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp target/release/shiftella-game "$APP/Contents/MacOS/shiftella-game"

cat > "$APP/Contents/MacOS/Shiftella" <<'EOF'
#!/bin/sh
HERE="$(cd "$(dirname "$0")" && pwd)"
cd "$HERE/../Resources" || exit 1
exec "$HERE/shiftella-game" "$@"
EOF
chmod +x "$APP/Contents/MacOS/Shiftella"

# Game resources
mkdir -p "$APP/Contents/Resources/assets"
for d in audio data fonts i18n icon; do
    [ -d "assets/$d" ] && cp -R "assets/$d" "$APP/Contents/Resources/assets/$d"
done

# Icon
ICONSET="$(mktemp -d)/AppIcon.iconset"
mkdir -p "$ICONSET"
python3 -I tools/make_icon.py assets/fonts/Inter-Bold.ttf "$ICONSET/icon_512x512@2x.png" --rgba assets/icon
for s in 16 32 128 256 512; do
    sips -z $s $s "$ICONSET/icon_512x512@2x.png" --out "$ICONSET/icon_${s}x${s}.png" >/dev/null
    d=$((s * 2))
    sips -z $d $d "$ICONSET/icon_512x512@2x.png" --out "$ICONSET/icon_${s}x${s}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/AppIcon.icns"

sed "s/__VERSION__/$VERSION/g" packaging/Info.plist > "$APP/Contents/Info.plist"

# Licences next to the game
cp assets/fonts/OFL.txt "$APP/Contents/Resources/OFL-Inter.txt"

echo "→ ad-hoc signature"
codesign --force --deep --sign - "$APP" >/dev/null

echo "→ zip"
rm -f dist/Shiftella-macos.zip
ditto -c -k --keepParent "$APP" dist/Shiftella-macos.zip

du -sh "$APP" dist/Shiftella-macos.zip
echo "done: $APP"

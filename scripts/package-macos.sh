#!/bin/sh
# Builds dist/Shiftella.app and dist/Shiftella-macos.zip.
#
#   ./scripts/package-macos.sh          # the release build
#   ./scripts/package-macos.sh --dev    # the developer build (docs/DEV-BUILD.md):
#                                       # dist/Shiftella Dev.app, dist/Shiftella-Dev-macos.zip,
#                                       # `--features dev`, bundle id app.shiftella.game.dev
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
NAME="Shiftella"
BUNDLE_ID="app.shiftella.game"
ZIP="dist/Shiftella-macos.zip"
FEATURES=""
for arg in "$@"; do
    case "$arg" in
        --dev)
            NAME="Shiftella Dev"
            BUNDLE_ID="app.shiftella.game.dev"
            ZIP="dist/Shiftella-Dev-macos.zip"
            FEATURES="--features dev"
            ;;
        *)
            echo "unknown option: $arg (use --dev)" >&2
            exit 2
            ;;
    esac
done
APP="dist/$NAME.app"

echo "→ release build v$VERSION ${FEATURES:+($FEATURES)}"
# shellcheck disable=SC2086
cargo build --release --bin shiftella-game $FEATURES

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
    # -L: a symlinked folder (the generated audio linked in from another checkout) is copied
    [ -d "assets/$d" ] && cp -RL "assets/$d" "$APP/Contents/Resources/assets/$d"
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

sed -e "s/__VERSION__/$VERSION/g" -e "s/__NAME__/$NAME/g" -e "s/__BUNDLE_ID__/$BUNDLE_ID/g" \
    packaging/Info.plist > "$APP/Contents/Info.plist"

# Licences next to the game
cp assets/fonts/OFL.txt "$APP/Contents/Resources/OFL-Inter.txt"

echo "→ ad-hoc signature"
codesign --force --deep --sign - "$APP" >/dev/null

echo "→ zip"
rm -f "$ZIP"
ditto -c -k --keepParent "$APP" "$ZIP"

du -sh "$APP" "$ZIP"
echo "done: $APP"

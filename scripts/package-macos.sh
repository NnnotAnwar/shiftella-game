#!/bin/sh
# Собирает dist/Shiftella.app и dist/Shiftella-macos.zip.
#
#   ./scripts/package-macos.sh
#
# Игра читает ресурсы по относительным путям (assets/…), поэтому исполняемый файл
# в бандле — маленький скрипт-запускатель: он переходит в Contents/Resources и
# запускает настоящий бинарник.
set -eu

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export PATH="/opt/homebrew/opt/rustup/bin:$HOME/.cargo/bin:$PATH"

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
APP="dist/Shiftella.app"

echo "→ release build v$VERSION"
cargo build --release --bin shiftella-game

# Переводы должны быть свежими
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

# Ресурсы игры (без исходников переводов и черновиков)
mkdir -p "$APP/Contents/Resources/assets"
for d in audio data fonts i18n; do
    cp -R "assets/$d" "$APP/Contents/Resources/assets/$d"
done

# Иконка
ICONSET="$(mktemp -d)/AppIcon.iconset"
mkdir -p "$ICONSET"
python3 -I tools/make_icon.py assets/fonts/Inter-Bold.ttf "$ICONSET/icon_512x512@2x.png"
for s in 16 32 128 256 512; do
    sips -z $s $s "$ICONSET/icon_512x512@2x.png" --out "$ICONSET/icon_${s}x${s}.png" >/dev/null
    d=$((s * 2))
    sips -z $d $d "$ICONSET/icon_512x512@2x.png" --out "$ICONSET/icon_${s}x${s}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/AppIcon.icns"

sed "s/__VERSION__/$VERSION/g" packaging/Info.plist > "$APP/Contents/Info.plist"

# Лицензии рядом с игрой
cp assets/fonts/OFL.txt "$APP/Contents/Resources/OFL-Inter.txt"

echo "→ ad-hoc signature"
codesign --force --deep --sign - "$APP" >/dev/null

echo "→ zip"
rm -f dist/Shiftella-macos.zip
ditto -c -k --keepParent "$APP" dist/Shiftella-macos.zip

du -sh "$APP" dist/Shiftella-macos.zip
echo "done: $APP"

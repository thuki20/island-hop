#!/usr/bin/env bash
# 生成正式的 macOS 应用包（.app，带自定义图标），可选再打 dmg。
#   bash packaging/macos-app.sh --app-only   # 只生成 .app，不产分发包
#   bash packaging/macos-app.sh              # 生成 .app + dist/*.dmg
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
APP="IslandHop.app"
APP_ONLY=0
[ "${1:-}" = "--app-only" ] && APP_ONLY=1

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp target/release/IslandHop "$APP/Contents/MacOS/IslandHop"
chmod +x "$APP/Contents/MacOS/IslandHop"
cp app/assets/AppIcon.icns "$APP/Contents/Resources/AppIcon.icns"

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>IslandHop</string>
  <key>CFBundleDisplayName</key><string>IslandHop</string>
  <key>CFBundleIdentifier</key><string>dev.islandhop.app</string>
  <key>CFBundleExecutable</key><string>IslandHop</string>
  <key>CFBundleIconFile</key><string>AppIcon.icns</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleVersion</key><string>${VERSION}</string>
  <key>CFBundleShortVersionString</key><string>${VERSION}</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST

# 注意：不再设置 LSUIElement=true —— 那会把 Dock 图标和菜单栏一起隐藏，
# 用户就看不到「应用」了。岛本身依旧无边框、置顶，且由 app 内部把
# NSWindow 抬到菜单栏之上（NSStatusWindowLevel = 25）。
codesign --force --deep --sign - "$APP" 2>/dev/null || true
echo "built $APP (icon: AppIcon.icns, Dock icon visible)"

if [ "$APP_ONLY" -eq 0 ]; then
  mkdir -p dist
  hdiutil create -volname IslandHop -srcfolder "$APP" -ov -format UDZO dist/IslandHop-macOS-arm64.dmg
  echo "built dist/IslandHop-macOS-arm64.dmg"
fi

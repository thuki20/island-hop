#!/usr/bin/env bash
set -euo pipefail

APP="IslandHop.app"
rm -rf "$APP" dist
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources" dist

cp target/release/IslandHop "$APP/Contents/MacOS/IslandHop"
chmod +x "$APP/Contents/MacOS/IslandHop"

cat > "$APP/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>IslandHop</string>
  <key>CFBundleDisplayName</key><string>IslandHop</string>
  <key>CFBundleIdentifier</key><string>dev.islandhop.app</string>
  <key>CFBundleExecutable</key><string>IslandHop</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleVersion</key><string>0.1.0</string>
  <key>CFBundleShortVersionString</key><string>0.1.0</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>LSUIElement</key><true/>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST

codesign --force --deep --sign - "$APP" 2>/dev/null || true
hdiutil create -volname IslandHop -srcfolder "$APP" -ov -format UDZO dist/IslandHop-macOS-arm64.dmg

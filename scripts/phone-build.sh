#!/usr/bin/env bash
# R211: builds the phone version into dist-phone, ready for static hosting.
# Needs: the wasm32-unknown-unknown target, wasm-pack, pnpm. BASE is the path it's served
# under (for GitHub Pages, /<repo>/).
set -euo pipefail
BASE="${BASE:-/study-app/}"
export RUSTFLAGS='--cfg getrandom_backend="wasm_js"'
wasm-pack build crates/study-phone --target web --release --out-dir pkg --no-typescript
VITE_PHONE=1 pnpm vite build --base "$BASE" --outDir dist-phone --emptyOutDir
mkdir -p dist-phone/phone-pkg
cp crates/study-phone/pkg/study_phone.js crates/study-phone/pkg/study_phone_bg.wasm dist-phone/phone-pkg/
cp src-tauri/icons/icon.png dist-phone/icon-512.png
cp src-tauri/icons/128x128@2x.png dist-phone/icon-256.png
cat > dist-phone/manifest.webmanifest <<JSON
{
  "name": "Shudhana",
  "short_name": "Shudhana",
  "start_url": "${BASE}",
  "scope": "${BASE}",
  "display": "standalone",
  "background_color": "#180f09",
  "theme_color": "#180f09",
  "icons": [
    { "src": "${BASE}icon-256.png", "sizes": "256x256", "type": "image/png" },
    { "src": "${BASE}icon-512.png", "sizes": "512x512", "type": "image/png" }
  ]
}
JSON
# Adding to the home screen: the manifest, the icon, and running full screen on iPhone.
sed -i "s|<title>Shudhana</title>|<title>Shudhana</title>\n    <link rel=\"manifest\" href=\"${BASE}manifest.webmanifest\" />\n    <link rel=\"apple-touch-icon\" href=\"${BASE}icon-256.png\" />\n    <meta name=\"apple-mobile-web-app-capable\" content=\"yes\" />\n    <meta name=\"apple-mobile-web-app-title\" content=\"Shudhana\" />\n    <meta name=\"theme-color\" content=\"#180f09\" />|" dist-phone/index.html
# GitHub Pages serves a 404 page for unknown paths. The app uses #routes, so this is just in case.
cp dist-phone/index.html dist-phone/404.html
echo "Phone version built in dist-phone"

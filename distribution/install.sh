#!/bin/sh
# Installs the Herdr app, and herdr-core if there's none yet:
#
#   curl -fsSL https://releases.herdr.dev/app/install.sh | sh
#
# Linux: the app into ~/.local/share/sunna, `sunna` in ~/.local/bin,
# a launcher entry and icon. macOS: Sunna.app into ~/Applications, keeping
# its release signature intact (ad hoc for unsigned/dev builds). The core, with its
# own installer, into ~/.local/bin/herdr-core (and `hc`) only if none is
# there yet; the app
# keeps it current after that. Running it again updates the app.
set -eu

releases=${HERDR_RELEASES_URL:-https://releases.herdr.dev}
bin="$HOME/.local/bin"

say() { printf '%s\n' "$*"; }
fail() { say "herdr: $*" >&2; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || fail "needs $1"; }
need curl
need tar

case "$(uname -s)" in
  Linux) os=linux ;;
  Darwin) os=macos ;;
  MINGW* | MSYS* | CYGWIN*) fail "this installer is for Linux and macOS; on Windows, run this in PowerShell: irm $releases/app/install.ps1 | iex" ;;
  *) fail "no build for $(uname -s)" ;;
esac
case "$(uname -m)" in
  x86_64 | amd64) arch=x86_64 ;;
  arm64 | aarch64) arch=aarch64 ;;
  *) fail "no build for $(uname -m)" ;;
esac
if [ "$os" = macos ] && [ "$arch" = x86_64 ]; then
  # Rosetta reports x86_64 even on Apple Silicon.
  if [ "$(sysctl -n hw.optional.arm64 2>/dev/null || true)" = 1 ] ||
     [ "$(sysctl -n sysctl.proc_translated 2>/dev/null || true)" = 1 ]; then
    arch=aarch64
  else
    fail "Intel Macs aren't supported; build from source: https://github.com/herdrdev/sunna#build-from-source"
  fi
fi
target="$os-$arch"

sha256() {
  if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | cut -d' ' -f1
  else shasum -a 256 "$1" | cut -d' ' -f1; fi
}

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

curl -fsSL "$releases/app/latest.json" -o "$tmp/latest.json" || fail "cannot reach $releases"
# Read without jq: the manifest is flat enough for sed once spaces are gone.
entry=$(tr -d '\n ' < "$tmp/latest.json" | sed -n "s/.*\"$target\":{\([^}]*\)}.*/\1/p")
url=$(printf '%s' "$entry" | sed -n 's/.*"url":"\([^"]*\)".*/\1/p')
want=$(printf '%s' "$entry" | sed -n 's/.*"sha256":"\([^"]*\)".*/\1/p')
version=$(tr -d '\n ' < "$tmp/latest.json" | sed -n 's/^{"version":"\([^"]*\)".*/\1/p')
[ -n "$url" ] && [ -n "$want" ] || fail "no build for $target yet"

# Tells the release server's download counter a new install from a reinstall;
# the file is the same either way. Only plain URLs on the release server in
# use are marked: one with a query or fragment (a mirror's signed URL) is
# fetched exactly as the manifest gives it.
src=install
for have in "${XDG_DATA_HOME:-$HOME/.local/share}/sunna" "$HOME/Applications/Sunna.app" /Applications/Sunna.app; do
  if [ -e "$have" ]; then src=reinstall; fi
done
case $url in
  *[?#]*) download=$url ;;
  "$releases"/*) download="$url?src=$src" ;;
  *) download=$url ;;
esac

say "Installing Herdr ${version:-} for $target..."
curl -fsSL "$download" -o "$tmp/app.tar.gz" || fail "download failed"
[ "$(sha256 "$tmp/app.tar.gz")" = "$want" ] || fail "download is damaged (checksum mismatch); try again"
tar -xzf "$tmp/app.tar.gz" -C "$tmp"
mkdir -p "$bin"

if [ "$os" = linux ]; then
  data=${XDG_DATA_HOME:-$HOME/.local/share}
  home="$data/sunna"
  mkdir -p "$data"
  rm -rf "$home.new"
  mv "$tmp/sunna" "$home.new"
  rm -rf "$home"
  mv "$home.new" "$home"
  rm -f "$bin/sunna"
  printf '#!/bin/sh\n# sunna cli launcher\nexec "%s" "$@"\n' "$home/sunna" > "$bin/sunna"
  chmod 755 "$bin/sunna"
  mkdir -p "$data/applications" "$data/icons/hicolor/scalable/apps"
  cp "$home/sunna.svg" "$data/icons/hicolor/scalable/apps/sunna.svg"
  cat > "$data/applications/sunna.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Herdr
Comment=Terminals and coding agents that keep running
Exec=$bin/sunna
Icon=sunna
Terminal=false
Categories=Development;
StartupWMClass=sunna
EOF
  command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$data/applications" 2>/dev/null || true
  [ -f "$data/icons/hicolor/icon-theme.cache" ] && gtk-update-icon-cache -f -t "$data/icons/hicolor" >/dev/null 2>&1 || true
  # Plasma remembers a missing icon until it is told the theme changed,
  # so a fresh install shows an empty page in the taskbar.
  command -v dbus-send >/dev/null 2>&1 &&
    dbus-send --session --type=signal /KIconLoader org.kde.KIconLoader.iconChanged int32:0 2>/dev/null || true
else
  apps="$HOME/Applications"
  app="$apps/Sunna.app"
  mkdir -p "$apps"
  # Prepared in the temp folder, so Spotlight never indexes a half-made
  # bundle under a temporary name in Applications.
  # Never change a certificate-signed bundle: macOS permissions follow its
  # signing identity. Only unsigned/ad-hoc development builds need this PATH.
  signed=false
  if signature=$(/usr/bin/codesign -dvv "$tmp/Sunna.app" 2>&1); then
    if printf '%s\n' "$signature" | grep -q '^Authority=Developer ID Application:'; then
      signed=true
    elif ! printf '%s\n' "$signature" | grep -qx 'Signature=adhoc'; then
      fail "cannot determine app signature: $signature"
    fi
  elif ! printf '%s\n' "$signature" | grep -q ': code object is not signed at all$'; then
    fail "cannot determine app signature: $signature"
  fi
  if [ "$signed" = true ]; then
    /usr/bin/codesign --verify --deep --strict "$tmp/Sunna.app" || fail "invalid app signature"
  else
    /usr/libexec/PlistBuddy -c "Add :LSEnvironment:PATH string $bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin" "$tmp/Sunna.app/Contents/Info.plist"
    /usr/bin/codesign --force --sign - "$tmp/Sunna.app" >/dev/null 2>&1
  fi
  rm -rf "$app"
  mv "$tmp/Sunna.app" "$app"
  rm -f "$bin/sunna"
  printf '#!/bin/sh\n# sunna cli launcher\nexec "%s" "$@"\n' "$app/Contents/MacOS/Sunna" > "$bin/sunna"
  chmod 755 "$bin/sunna"
  /System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f "$app" 2>/dev/null || true
fi

# The core: installed only if there's none yet. One that's already there
# (yours, or from a package manager) is left alone; the app updates it.
if [ ! -e "$bin/herdr-core" ] && ! command -v herdr-core >/dev/null 2>&1; then
  curl -fsSL "$releases/core/install.sh" | HC_INSTALL_DIR="$bin" HERDR_RELEASES_URL="$releases" sh || say "herdr-core didn't install; Herdr will offer it when it opens."
fi

say "Herdr is installed. Open it from your apps, or run: sunna"
case ":$PATH:" in
  *":$bin:"*) ;;
  *) say "Add $bin to your PATH to use sunna and hc from a terminal." ;;
esac

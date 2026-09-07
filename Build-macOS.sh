#!/usr/bin/env bash

set -euo pipefail

project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
log_path="$project_root/build.log"

fail() {
    printf '\nBUILD FAILED\n'
    printf '%s\n' "$1"
    printf '\nFull output: %s\n' "$log_path"
    if [[ "${NO_PAUSE:-0}" != "1" ]]; then
        read -r -p "Press Enter to close"
    fi
    exit 1
}

trap 'fail "The build stopped unexpectedly."' ERR

: > "$log_path"
exec > >(tee -a "$log_path") 2>&1

cd "$project_root"

for command in node npm cargo ditto; do
    if ! command -v "$command" >/dev/null 2>&1; then
        fail "$command was not found. Install Node.js LTS, Rust stable, and the Xcode command line tools, then reopen Terminal."
    fi
done

printf 'Installing JavaScript dependencies...\n'
npm install

printf 'Building the macOS binary...\n'
npm exec tauri -- build --no-bundle

printf 'Bundling the macOS app...\n'
npm exec tauri -- bundle --bundles app

app_bundle="$project_root/src-tauri/target/release/bundle/macos/STRM Inspector.app"
if [[ ! -d "$app_bundle" ]]; then
    fail "Tauri completed without creating $app_bundle."
fi

dist_dir="$project_root/dist"
zip_path="$dist_dir/STRM-Inspector-Mac.zip"

mkdir -p "$dist_dir"
rm -f "$zip_path"

printf 'Creating shareable ZIP...\n'
ditto -c -k --sequesterRsrc --keepParent "$app_bundle" "$zip_path"

printf '\nBUILD COMPLETE\n'
printf 'App bundle: %s\n' "$app_bundle"
printf 'Shareable ZIP: %s\n' "$zip_path"

if [[ "${NO_PAUSE:-0}" != "1" ]]; then
    read -r -p "Press Enter to close"
fi

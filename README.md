# STRM Inspector

STRM Inspector is a Tauri 2 desktop app for opening, inspecting, and exporting `.strm` image streams.

## What it does

- Loads STRM version 2 files
- Decodes LZ4-compressed BC7 frame payloads in Rust
- Reads section metadata and named frame ranges
- Shows stream details such as dimensions, frame count, frame rate, and compression type
- Plays frames inside the viewer
- Exports the current frame as PNG
- Exports selected sections as PNG sequences

## Project layout

- `src/` contains the React UI
- `src-tauri/` contains the Rust backend and STRM parsing/decoding logic
- `Build-Windows.ps1` and `Build-Windows.cmd` are helper scripts for Windows packaging
- `Build-macOS.sh` and `Build-macOS.command` are helper scripts for macOS packaging

## Requirements

For development you need:

- Node.js
- Rust toolchain
- Tauri prerequisites for your platform

On Windows, you also need WebView2 and the native build tools required by Tauri.
On macOS, you need Xcode command line tools and the usual Tauri prerequisites for macOS builds.

## Development

Install dependencies and start the desktop app:

```powershell
npm install
npm run tauri dev
```

If you only want the web UI during development:

```powershell
npm run dev
```

## Build

Build the app with:

```powershell
npm run build
```

For desktop packaging:

- Windows: `npm run build:windows` or run `Build-Windows.ps1`
- macOS: `npm run build:mac` or double-click `Build-macOS.command`

## Export output

Exported section folders preserve the original frame order. Each exported PNG filename includes both the local sequence number and the source frame index so overlapping or repeated ranges stay unambiguous.

## Format support

The current implementation expects:

- STRM file version 2
- LZ4 block-compressed frame data
- BC7 texture payloads
- Section metadata version 1

## License

No project license file is currently included. Add one if you plan to publish or share the project more broadly.

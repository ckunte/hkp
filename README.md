# hkp

A collection of housekeeping tools (`~/scripts/hkp`) to manage files, folders, repos, set permissions, combine PDF files, and serve a directory over HTTP — packaged as a single `hk` binary written in Rust.

## usage

```
hk — housekeeping tools

usage: hk <command> [argument]

commands:
  cmbi        combine all PDF files in the current folder
  cmbs        combine PDF files per subdirectory
  cpng        compress PNG images into ./compressed/     [requires: pngquant]
  cr [--yes]  squash git history and force-push (asks first) [requires: git]
  ffp [--yes] [--all]
              fix permissions: dirs 755, files 644 (keeps +x, skips private)
  sffn        sanitise folder and file names
  srv [port] [--public]
              serve the current folder over HTTP (default port 8000,
              localhost only; --public listens on all interfaces)
  help        show this help
```

## install

### prebuilt binaries

Download the archive for your system from [Releases](https://github.com/ckunte/hkp/releases), extract it, and put `hk` (or `hk.exe`) somewhere on your `PATH`:

| system                              | archive                          |
|-------------------------------------|----------------------------------|
| Windows (x86-64)                    | `hk-<version>-windows-x86_64.zip`  |
| macOS (Apple silicon, M-series)     | `hk-<version>-macos-arm64.tar.gz`  |
| Raspberry Pi 5 (64-bit Pi OS/Linux) | `hk-<version>-linux-arm64-rpi5.tar.gz` |
| Linux x86-64 (any distro)           | `hk-<version>-linux-x86_64.tar.gz` |

On macOS, clear the quarantine flag after downloading: `xattr -d com.apple.quarantine hk`

### from source

Rust is required to build `hk`. Install it via rustup:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Then build and install the binary:

```bash
cargo install --path ~/scripts/hkp/hk
```

After that, `hk <command>` is available anywhere.

## dependencies

Only `cpng` and `cr` require external tools:

| command | requires | install |
|---------|----------|---------|
| `cpng`  | pngquant | see below |
| `cr`    | git      | https://git-scm.com |

**pngquant** — install for your platform:

```
macOS (Homebrew)    brew install pngquant
Linux (apt)         sudo apt install pngquant
Linux (dnf)         sudo dnf install pngquant
Windows (winget)    winget install pngquant
Windows (Scoop)     scoop install pngquant
All platforms       https://pngquant.org
```

## notes

- `cmbi` and `cmbs` use pure Rust (lopdf) — no Ghostscript needed for combining PDFs
- `srv` is a built-in HTTP server — no Python needed. It binds to 127.0.0.1, serves GET/HEAD only, hides dotfiles, and refuses symlinks that point outside the served folder
- `cr` requires a clean working tree and a typed confirmation (or `--yes`), and uses `--force-with-lease`; the original branch is restored if the push fails
- `sffn` skips any rename whose target already exists, and reports it
- `sffn` sanitises folder names and the names of document (pdf, doc/docx, xls/xlsx, ppt/pptx, odt/ods/odp, rtf, txt, md, csv, epub, pages/numbers/key), image (jpg, png, gif, webp, heic, tiff, bmp, svg), audio (mp3, wav, flac, m4a, aac, ogg) and video (mp4, mov, mkv, avi, webm, m4v) files natively — no fd, detox, or rename needed. Other files and hidden files are left alone. The list is `SANITISE_EXTS` in `src/main.rs`
- `ffp` is pure Rust — no shell tools needed (macOS / Linux only; not available on Windows). It asks for confirmation (`--yes` to skip), refuses to run in `/` or your home folder, keeps the executable bit on files that already have it (755), and skips private entries with no group/other access (such as 600 keys) unless you pass `--all`

## releases

Release archives are built from a tagged commit, listed with checksums in `SHA256SUMS.txt`, and carry signed build provenance. Verify a download with:

```bash
gh attestation verify hk-<version>-<system>.tar.gz --repo ckunte/hkp
```

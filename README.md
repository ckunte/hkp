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
  cr          squash git history (new orphan branch)     [requires: git]
  ffp         fix permissions: dirs 755, files 644
  sffn        sanitise folder and file names
  srv [port]  serve the current folder over HTTP (default port 8000)
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
- `srv` is a built-in HTTP server — no Python needed
- `sffn` sanitises file/folder names natively — no fd, detox, or rename needed
- `ffp` is pure Rust — no shell tools needed (macOS / Linux only; not available on Windows)

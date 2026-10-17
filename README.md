# hkp

A collection of housekeeping tools to manage files and folders, set permissions, combine and crop PDF files — packaged as a single `hk` binary written in Rust.

## usage

```
hk — housekeeping tools

usage: hk <command> [argument]

commands:
  cmbi   combine all PDF files in the current folder
  cmbs   combine PDF files per subfolder
  crop   crop PDF header/footer (--top/--bott N inches, default 1.0) file1.pdf file2.pdf
  fffp [--yes] [--all]
         fix permissions: dirs 755, files 644 (keeps +x, skips private)
  sffn   sanitise folder and file names
  help   show this help
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

The binaries are not code-signed or notarized, so macOS will block `hk` on first run. Before overriding that, verify the download (see [releases](#releases)), then clear the quarantine flag: `xattr -d com.apple.quarantine hk`.

For macOS, a bash install script [mac-inst.sh][mi] is provided, which does the following:

1. Downloads the latest macOS (Apple Silicon) release
1. Verifies the download against its hash in `SHA256SUMS.txt`
1. If the hash matches, extracts the `hk` binary (otherwise stops)
1. Clears the macOS quarantine flag
1. Installs `hk` to `/usr/local/bin` (uses `sudo` if needed)

Run it as follows from a Terminal (be sure to check the script, if unsure):

```bash
curl --proto '=https' --tlsv1.2 -sSfL https://github.com/ckunte/hkp/raw/master/mac-inst.sh | bash
```

[mi]: https://github.com/ckunte/hkp/blob/master/mac-inst.sh

### from source

Rust is required to build `hk`. Install it via rustup:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Then, from a clone of this repository, build and install the binary:

```bash
cargo install --path . --locked
```

After that, `hk <command>` is available anywhere.

## dependencies

None — `hk` is a single self-contained binary.

## notes

- `cmbi` and `cmbs` use pure Rust (lopdf) — no Ghostscript needed for combining PDFs. Re-running them replaces the previous `*-comb.pdf` without merging it into itself; the output is written atomically, inputs over 1 GiB or malformed files are reported as errors, and `cmbs` carries on past a failing folder
- `sffn` skips any rename whose target already exists, and reports it
- `sffn` sanitises folder names and the names of document (pdf, doc/docx, typ, xls/xlsx, ppt/pptx, odt/ods/odp, rtf, txt, md, csv, epub, pages/numbers/key), image (jpg, png, gif, webp, heic, tiff, bmp, svg), audio (mp3, wav, flac, m4a, aac, ogg) and video (mp4, mov, mkv, avi, webm, m4v) files natively — no fd, detox, or rename needed. Other files and hidden files are left alone. The list is `SANITISE_EXTS` in `src/main.rs`
- `fffp` is pure Rust — no shell tools needed (macOS / Linux only; not available on Windows). It asks for confirmation (`--yes` to skip), refuses to run in `/` or your home folder, keeps the executable bit on files that already have it (755), and skips private entries with no group/other access (such as 600 keys) unless you pass `--all`

## releases

Release archives are built from a tagged commit, listed with checksums in `SHA256SUMS.txt`, and carry signed build provenance. Verify a download with:

```bash
gh attestation verify hk-<version>-<system>.tar.gz --repo ckunte/hkp
```

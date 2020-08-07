# hkp

A collection of my scripts (`~/hkp`) to manage files, folders, repos, set permissions, prune empty folders, combine pdf files, etc. Requires [fd], ghostscript, [detox][d], rename, and python. This includes:

1. Combine all pdf files and the current folder
2. Combine subfolder-wise pdf files
3. Purge empty folders
4. Apply file and folder permissions
5. Purge LaTeX compile files
6. Compress all pdf files in the current folder
7. Delete and recreate master git repository
8. Sanitise folder and filenames

## usage

list available scripts:

```bash
make --file=/path/to/hkp.make
```

run a script:

```bash
make --file=~/path/to/hkp.make <cmd>
```

simplify the long command: by adding the following to `.bashrc` (or `.zshrc`):

```bash
alias hk="make --file=/path/to/hkp.make"
```

With the above the following can simply be used:

```bash
hk <cmd>
```

[d]: https://github.com/dharple/detox
[fd]: https://github.com/sharkdp/fd "A simple, fast and friendly alternative to find"

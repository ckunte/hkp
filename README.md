# hkp

A collection of my scripts (`~/scripts/hkp`) to manage files, folders, repos, set permissions, prune empty folders, combine pdf files, etc.

## usage

list available scripts:

```bash
usage: make --file=~/scripts/hkp/hkp.make <cmd>
requires detox, fd, git, gs, pandoc, python{,3}, rename, zsh
<cmd> available:
 cmbi   combine all pdf files in the curr folder
 cmbs   combine sub-folder-wise pdf files
 cmpr   compress all pdf files in the curr folder
 cr     delete and recreate master git repository
 dss    delete all .DS_Store files
 efd    purge empty folders
 ffp    apply permissions: folders (755), files (644)
 help   show this help
 m2t    convert all .md files to text in the curr folder
 plcf   purge LaTeX compile files
 sffn   sanitise folder and filenames
 srv    serve this folder
```
run a script:

```bash
make --file=~~/scripts/hkp/hkp.make <cmd>
```

simplify the long command: by adding the following to `.bashrc` (or `.zshrc`):

```bash
alias hk="make --file=~/scripts/hkp/hkp.make"
```

With the above the following can simply be used:

```bash
hk <cmd>
```

## requires

This package requires detox, fd, git, gs, pandoc, python, rename, and zsh. With Homebrew, these can be installed as follows:

```bash
brew update && brew install detox fd git gs pandoc python3 rename zsh
```

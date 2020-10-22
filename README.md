# hkp

A collection of my scripts (`~/scripts/hkp`) to manage files, folders, repos, set permissions, prune empty folders, combine pdf files, etc.

## usage

list available scripts:

```bash
make --file=~/scripts/hkp/hkp.make
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

This package requires fd, git, gs, pandoc, python, rename, and zsh. With Homebrew, these can be installed as follows:

```bash
brew update && brew install fd git gs pandoc python3 rename zsh
```
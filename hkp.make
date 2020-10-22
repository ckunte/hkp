# Makefile for all scripts in ~/scripts/hkp/ folder
# 2019 ckunte

.PHONY: help
help: ## show this help
	@echo 'usage: make --file=~/scripts/hkp/hkp.make <cmd>'
	@echo 'requires detox, fd, git, gs, pandoc, python{,3}, rename, zsh'
	@echo '<cmd> available:'
	@grep -e '\s##\s' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf " \033[36m%-6s\033[0m %s\n", $$1, $$2}'

dss: ## delete all .DS_Store files
	@bash ~/scripts/hkp/dss.sh

m2t: ## convert all .md files to text in the curr folder
	@bash ~/scripts/hkp/md2txt.sh

cmbi: ## combine all pdf files in the curr folder
	@bash ~/scripts/hkp/cmb.sh

cmbs: ## combine sub-folder-wise pdf files
	@python3 ~/scripts/hkp/cmb.py

cmpr: ## compress all pdf files in the curr folder
	@detox ./*
	@bash ~/scripts/hkp/cmpr.sh

cr: ## delete and recreate master git repository
	@bash ~/scripts/hkp/cr.sh

efd: ## purge empty folders
	@bash ~/scripts/hkp/efd.sh

ffp: ## apply permissions: folders (755), files (644)
	@bash ~/scripts/hkp/ffp.sh

plcf: ## purge LaTeX compile files
	@bash ~/scripts/hkp/plcf.sh

sffn: ## sanitise folder and filenames
	@detox -r ./*
	@bash ~/scripts/hkp/rdot.sh
	@fd --type f . | rename 's/-\.pdf/\.pdf/g'

srv: ## serve this folder
	@python3 ~/scripts/hkp/srv3.py
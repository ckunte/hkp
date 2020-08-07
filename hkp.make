# Makefile for all scripts in hkp folder
# 2019 ckunte

.PHONY: help
help: ## show this help
	@echo 'usage: make --file=/path/to/hkp.make <cmd>'
	@echo '<cmd> available:'
	@egrep -h '\s##\s' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf " \033[36m%-6s\033[0m %s\n", $$1, $$2}'

m2t: ## convert all markdown files to text in the folder
	@bash ~/hkp/md2txt.sh

cmbi: ## combine all pdf files in the curr folder
	@bash ~/hkp/cmb.sh

cmbs: ## combine sub-folder-wise pdf files
	@python ~/hkp/cmb.py

cmpr: ## compress all pdf files in the curr folder
	@detox ./*
	@bash ~/hkp/cmpr.sh

cr: ## delete and recreate master git repository
	@bash ~/hkp/cr.sh

efd: ## purge empty folders
	@bash ~/hkp/efd.sh

ffp: ## apply permissions: folders (755), files (644)
	@bash ~/hkp/ffp.sh

plcf: ## purge LaTeX compile files
	@bash ~/hkp/plcf.sh

sffn: ## sanitise folder and filenames
	@detox -r ./*
	@bash ~/hkp/rdot.sh
	@fd --type f . | rename 's/-\.pdf/\.pdf/g'

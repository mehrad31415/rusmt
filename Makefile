# help
define cmdline
Please choose one of the specific commands:
  - lint	: lint and format the code
  - cloc	: count total number of lines of code
  - docs	: build and display the documentation
endef
export cmdline

help:
	@echo "$$cmdline"

lint:
	@cargo fmt && \
	cargo clippy --all-targets --all-features

cloc:
	@cloc \
		--include-lang=Rust \
		smt \
		programs \
		lang

docs:
	@cd book && \
		mdbook clean && mdbook build && mdbook serve --open

.PHONY: help lint cloc docs

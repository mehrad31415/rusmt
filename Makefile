# help
define cmdline
Please choose one of the specific commands:
  - lint	: lint and format the code
  - cloc	: count total number of lines of code
  - docs	: build and display the documentation
  - rego	: semantics for language: rego
endef
export cmdline

help:
	@echo "$$cmdline"

lint:
	@cargo fmt && \
	cargo clippy --all-targets --all-features

cloc:
	@cloc \
		utils \
		smt \
		evaluation \
		lang \

docs:
	@cd doc/book && \
		mdbook clean && mdbook build && mdbook serve --open

rego:
	@cd lang/src/rego && \
	cargo run rego

.PHONY: help lint cloc docs rego

CARGO = cargo
BIN = git-editor
TARGET_DIR = target/release
DOCKER = docker
PREFIX ?= /usr/local
REPO_PATH ?= $(CURDIR)

# Default target
.PHONY: all
all: build

# Build the project
.PHONY: build
build:
	$(CARGO) build --release --locked

# Show the history of REPO_PATH (read-only; defaults to this directory)
.PHONY: run
run:
	$(CARGO) run --release -- --show-history --repo-path "$(REPO_PATH)"

# Preview a full rewrite of REPO_PATH without changing anything
# (set EMAIL, NAME, START, END)
.PHONY: run-custom
run-custom:
	$(CARGO) run --release -- --simulate --show-diff \
		--repo-path "$(REPO_PATH)" \
		--email "$(EMAIL)" \
		--name "$(NAME)" \
		--begin "$(START)" \
		--end "$(END)"

# Really rewrite REPO_PATH (must be given explicitly; asks for confirmation)
.PHONY: rewrite
rewrite:
	@test -n "$(filter command line,$(origin REPO_PATH))" || { echo "Set REPO_PATH=... explicitly to rewrite a repository"; exit 1; }
	$(CARGO) run --release -- \
		--repo-path "$(REPO_PATH)" \
		--email "$(EMAIL)" \
		--name "$(NAME)" \
		--begin "$(START)" \
		--end "$(END)"

# Clean build artifacts
.PHONY: clean
clean:
	$(CARGO) clean

# Run tests
.PHONY: test
test:
	$(CARGO) test

# Check code formatting
.PHONY: fmt
fmt:
	$(CARGO) fmt --all -- --check

# Lint the code (same flags as CI)
.PHONY: lint
lint:
	$(CARGO) clippy --all-targets --all-features -- -D warnings

# Docker build
.PHONY: docker-build
docker-build:
	$(DOCKER) build -t $(BIN):latest .

# Show the history of REPO_PATH from the Docker image (read-only)
.PHONY: docker-run
docker-run:
	$(DOCKER) run --rm -it \
		--user "$$(id -u):$$(id -g)" -e HOME=/tmp \
		-v "$(REPO_PATH)":/workspace \
		$(BIN):latest --show-history

# Install the binary (override PREFIX/DESTDIR as needed, e.g. PREFIX=~/.local)
.PHONY: install
install: build
	install -Dm755 $(TARGET_DIR)/$(BIN) "$(DESTDIR)$(PREFIX)/bin/$(BIN)"

# Uninstall the binary
.PHONY: uninstall
uninstall:
	rm -f "$(DESTDIR)$(PREFIX)/bin/$(BIN)"

# Help
.PHONY: help
help:
	@echo "Git Editor Make Commands:"
	@echo "  all             - Build the project (alias for build)"
	@echo "  build           - Build the release binary"
	@echo "  run             - Show the history of REPO_PATH (read-only, default: current dir)"
	@echo "  run-custom      - Preview a full rewrite (set REPO_PATH, EMAIL, NAME, START, END)"
	@echo "  rewrite         - Rewrite REPO_PATH for real (REPO_PATH must be set explicitly)"
	@echo "  clean           - Clean build artifacts"
	@echo "  test            - Run tests"
	@echo "  fmt             - Check code formatting"
	@echo "  lint            - Run clippy with CI flags"
	@echo "  docker-build    - Build Docker image"
	@echo "  docker-run      - Show the history of REPO_PATH from the Docker image"
	@echo "  install         - Install binary to \$$(DESTDIR)\$$(PREFIX)/bin (default /usr/local/bin)"
	@echo "  uninstall       - Remove the installed binary"
	@echo "  help            - Show this help message"

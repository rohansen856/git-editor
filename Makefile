CARGO = cargo
BIN = git-editor
TARGET_DIR = target/release
ENV_FILE = .env
DOCKER = docker

# Default target
.PHONY: all
all: build

# Build the project
.PHONY: build
build:
	$(CARGO) build --release

# Run the project with default settings
.PHONY: run
run:
	$(CARGO) run --release -- \
		--repo-path $(shell pwd) \
		--email "user@example.com" \
		--name "User Name" \
		--start "2023-01-01 00:00:00" \
		--end "2023-01-07 23:59:59"

# Run the project with custom settings (specify via environment variables)
.PHONY: run-custom
run-custom:
	$(CARGO) run --release -- \
		--repo-path "$(REPO_PATH)" \
		--email "$(EMAIL)" \
		--name "$(NAME)" \
		--start "$(START)" \
		--end "$(END)"

# Check if .env exists, if not, create from example
$(ENV_FILE):
	@if [ ! -f $(ENV_FILE) ]; then \
		cp .env.example $(ENV_FILE); \
		echo "Created $(ENV_FILE) from example. Please update with your credentials."; \
	fi

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

# Lint the code
.PHONY: lint
lint:
	$(CARGO) clippy -- -D warnings

# Docker build
.PHONY: docker-build
docker-build:
	$(DOCKER) build -t $(BIN):latest .

# Docker run
.PHONY: docker-run
docker-run: $(ENV_FILE)
	$(DOCKER) run --rm -it \
		--env-file $(ENV_FILE) \
		-v $(shell pwd):/workspace \
		$(BIN):latest \
		--repo-path "/workspace" \
		--email "user@example.com" \
		--name "User Name" \
		--start "2023-01-01 00:00:00" \
		--end "2023-01-07 23:59:59"

# Install the binary to system path
.PHONY: install
install: build
	cp $(TARGET_DIR)/$(BIN) /usr/local/bin/

# Uninstall the binary
.PHONY: uninstall
uninstall:
	rm -f /usr/local/bin/$(BIN)

# Get contribution data via GitHub API
.PHONY: contributions
contributions: $(ENV_FILE)
	@echo "Fetching GitHub contribution data..."
	@$(CARGO) run --release -- --fetch-contributions

# Help
.PHONY: help
help:
	@echo "Git Editor Make Commands:"
	@echo "  all             - Build the project (alias for build)"
	@echo "  build           - Build the release binary"
	@echo "  run             - Run with default settings"
	@echo "  run-custom      - Run with custom settings (set REPO_PATH, EMAIL, NAME, START, END env vars)"
	@echo "  clean           - Clean build artifacts"
	@echo "  test            - Run tests"
	@echo "  fmt             - Check code formatting"
	@echo "  lint            - Run clippy linter"
	@echo "  docker-build    - Build Docker image"
	@echo "  docker-run      - Run in Docker container"
	@echo "  install         - Install binary to /usr/local/bin"
	@echo "  uninstall       - Remove binary from /usr/local/bin"
	@echo "  contributions   - Fetch GitHub contribution data"
	@echo "  help            - Show this help message"
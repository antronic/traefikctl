BINARY     := traefikctl
DIST       := dist
VERSION    := $(shell grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
DOCKER_IMG := rust:1.89-slim

# Detect host
HOST_OS    := $(shell uname -s | tr A-Z a-z)
HOST_ARCH  := $(shell uname -m)

ifeq ($(HOST_ARCH),arm64)
  HOST_ARCH := aarch64
endif
ifeq ($(HOST_ARCH),x86_64)
  HOST_ARCH := x86_64
endif

.PHONY: build build-native build-linux-x86 build-linux-arm build-darwin-x86 clean all

build: build-native

build-native:
	cargo build --release
	@mkdir -p $(DIST)/$(HOST_OS)-$(HOST_ARCH)
	cp target/release/$(BINARY) $(DIST)/$(HOST_OS)-$(HOST_ARCH)/$(BINARY)
	@echo "→ $(DIST)/$(HOST_OS)-$(HOST_ARCH)/$(BINARY)"

build-linux-x86:
	docker run --rm --platform linux/amd64 \
		-v "$(CURDIR)":/app -w /app \
		$(DOCKER_IMG) cargo build --release
	@mkdir -p $(DIST)/linux-x86_64
	cp target/release/$(BINARY) $(DIST)/linux-x86_64/$(BINARY)
	@echo "→ $(DIST)/linux-x86_64/$(BINARY)"

build-linux-arm:
	docker run --rm --platform linux/arm64 \
		-v "$(CURDIR)":/app -w /app \
		$(DOCKER_IMG) cargo build --release
	@mkdir -p $(DIST)/linux-aarch64
	cp target/release/$(BINARY) $(DIST)/linux-aarch64/$(BINARY)
	@echo "→ $(DIST)/linux-aarch64/$(BINARY)"

build-darwin-x86:
	@if [ "$(HOST_OS)" != "darwin" ]; then echo "Error: macOS cross-compile requires macOS host"; exit 1; fi
	@if ! rustup target list --installed 2>/dev/null | grep -q x86_64-apple-darwin; then \
		echo "Adding x86_64-apple-darwin target..."; \
		rustup target add x86_64-apple-darwin; \
	fi
	cargo build --release --target x86_64-apple-darwin
	@mkdir -p $(DIST)/darwin-x86_64
	cp target/x86_64-apple-darwin/release/$(BINARY) $(DIST)/darwin-x86_64/$(BINARY)
	@echo "→ $(DIST)/darwin-x86_64/$(BINARY)"

all: build-native build-linux-x86

clean:
	cargo clean
	rm -rf $(DIST)

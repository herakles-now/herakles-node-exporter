.PHONY: build release install-binary clean help

# Allow passing additional cargo flags via CARGOFLAGS variable
CARGOFLAGS ?=

# Default target
help:
	@echo "Available targets:"
	@echo "  make build          - Build debug binary and copy to binary/"
	@echo "  make release        - Build release binary and copy to binary/"
	@echo "  make clean          - Clean build artifacts"
	@echo "  make install-binary - Copy binary to binary/ (called automatically after build)"
	@echo ""
	@echo "Examples:"
	@echo "  make build CARGOFLAGS='--no-default-features'"
	@echo "  make release CARGOFLAGS='--features ebpf'"

# Build debug binary and copy to binary/
build:
	cargo build $(CARGOFLAGS)
	@$(MAKE) install-binary PROFILE=debug

# Build release binary and copy to binary/
release:
	cargo build --release $(CARGOFLAGS)
	@$(MAKE) install-binary PROFILE=release

# Copy binary to binary/ directory
install-binary:
	@mkdir -p binary
	@if [ -f "target/$(PROFILE)/herakles-node-exporter" ]; then \
		cp target/$(PROFILE)/herakles-node-exporter binary/herakles-node-exporter; \
		chmod +x binary/herakles-node-exporter; \
		echo "✓ Binary copied to binary/herakles-node-exporter"; \
	else \
		echo "✗ Binary not found at target/$(PROFILE)/herakles-node-exporter"; \
		exit 1; \
	fi

# Clean build artifacts
clean:
	cargo clean
	rm -rf binary/

CLANG_BUILTIN := $(shell dirname $$(find /nix/store -maxdepth 6 -name 'stdbool.h' -path '*clang*include*' 2>/dev/null | head -1) 2>/dev/null)
SYSROOT := $(CURDIR)/patches/wasm-sysroot/include
WASM_TARGET := wasm32-unknown-unknown
PKG_DIR := $(CURDIR)/web/pkg
RELEASE_WASM := $(CURDIR)/target/$(WASM_TARGET)/release/helix_wasm.wasm

export CC_wasm32_unknown_unknown := clang-19
export CFLAGS_wasm32_unknown_unknown := --target=$(WASM_TARGET) -Os -nostdinc -isystem $(CLANG_BUILTIN) -isystem $(SYSROOT)
export RUSTFLAGS := --cfg tokio_unstable --cfg getrandom_backend="wasm_js"

.PHONY: wasm wasm-debug wasm-bindgen wasm-opt web serve clean

wasm: ## Build release WASM binary
	cargo build --target $(WASM_TARGET) -p helix-wasm --release

wasm-debug: ## Build debug WASM binary
	cargo build --target $(WASM_TARGET) -p helix-wasm

wasm-bindgen: wasm ## Run wasm-bindgen to generate JS bindings
	@mkdir -p $(PKG_DIR)
	wasm-bindgen --target web --out-dir $(PKG_DIR) $(RELEASE_WASM)

wasm-opt: wasm-bindgen ## Optimize WASM binary
	wasm-opt -Oz $(PKG_DIR)/helix_wasm_bg.wasm -o $(PKG_DIR)/helix_wasm_bg.wasm

web: wasm-opt ## Full web build (compile + bindgen + optimize)
	@echo ""
	@echo "Build complete: $$(du -h $(PKG_DIR)/helix_wasm_bg.wasm | cut -f1)"
	@echo "Serve with: make serve"

serve: ## Serve the web frontend on :8080
	cd web && python3 -m http.server 8080

native: ## Build native helix (unchanged)
	cargo build -p helix-term

clean: ## Clean WASM build artifacts
	cargo clean --target $(WASM_TARGET)
	rm -rf $(PKG_DIR)

help: ## Show this help
	@grep -E '^[a-z_-]+:.*##' $(MAKEFILE_LIST) | awk -F ':.*## ' '{printf "  %-16s %s\n", $$1, $$2}'

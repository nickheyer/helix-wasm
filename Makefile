CLANG_BUILTIN := $(shell dirname $$(find /nix/store -maxdepth 6 -name 'stdbool.h' -path '*clang*include*' 2>/dev/null | head -1) 2>/dev/null)
SYSROOT := $(CURDIR)/patches/wasm-sysroot/include
WASM_TARGET := wasm32-unknown-unknown
PKG_DIR := $(CURDIR)/web/pkg
RELEASE_WASM := $(CURDIR)/target/$(WASM_TARGET)/release/helix_wasm.wasm

# ── Toolchain ─────────────────────────────────────────────────────────────
CC       := clang-19
CXX      := clang++
LD       := wasm-ld
WASM_CC  := $(CC) --target=$(WASM_TARGET) -fPIC -Os -nostdlib \
            -fno-builtin -fno-exceptions -fno-threadsafe-statics \
            -nostdinc -isystem $(CLANG_BUILTIN) -isystem $(SYSROOT)
WASM_CXX := $(CXX) --target=$(WASM_TARGET) -fPIC -Os -nostdlib \
            -fno-builtin -fno-exceptions -fno-rtti -fno-threadsafe-statics \
            -nostdinc -isystem $(CLANG_BUILTIN) -isystem $(SYSROOT)

export CC_wasm32_unknown_unknown := $(CC)
export CFLAGS_wasm32_unknown_unknown := --target=$(WASM_TARGET) -Os -nostdinc -isystem $(CLANG_BUILTIN) -isystem $(SYSROOT)
# HOST_CC: cc-rs uses this for host-targeted builds during cross-compilation.
# The nix devshell exports CC=clang-19 (bare binary, missing builtin headers).
# The nix `cc` wrapper knows its own include paths.
export HOST_CC := cc
export RUSTFLAGS := --cfg tokio_unstable --cfg getrandom_backend="wasm_js" -C link-arg=--growable-table

# ── Grammar directories ──────────────────────────────────────────────────
GRAMMAR_SOURCES := $(CURDIR)/runtime/grammars/sources
GRAMMAR_OUT     := $(CURDIR)/web/grammars
# Discover all grammar source dirs that have a parser.c
GRAMMAR_NAMES   := $(notdir $(patsubst %/src/parser.c,%,$(wildcard $(GRAMMAR_SOURCES)/*/src/parser.c)))

.PHONY: wasm wasm-debug wasm-bindgen wasm-opt web serve clean grammars grammar-clean help

# ── Main targets ──────────────────────────────────────────────────────────

wasm: ## Build release WASM binary
	cargo build --target $(WASM_TARGET) -p helix-wasm --release

wasm-debug: ## Build debug WASM binary
	cargo build --target $(WASM_TARGET) -p helix-wasm

wasm-bindgen: wasm ## Run wasm-bindgen to generate JS bindings
	@mkdir -p $(PKG_DIR)
	wasm-bindgen --target web --out-dir $(PKG_DIR) $(RELEASE_WASM)

wasm-opt: wasm-bindgen ## Optimize WASM binary
	wasm-opt -Oz $(PKG_DIR)/helix_wasm_bg.wasm -o $(PKG_DIR)/helix_wasm_bg.wasm

web-debug: wasm-bindgen ## Debug web build (no wasm-opt, preserves function names)
	@echo ""
	@echo "DEBUG build: $$(du -h $(PKG_DIR)/helix_wasm_bg.wasm | cut -f1)"
	@echo "Serve with: make serve"

web: wasm-opt ## Full web build (compile + bindgen + optimize)
	@echo ""
	@echo "Build complete: $$(du -h $(PKG_DIR)/helix_wasm_bg.wasm | cut -f1)"
	@echo "Grammars: $$(ls $(GRAMMAR_OUT)/*.wasm 2>/dev/null | wc -l) .wasm files"
	@echo "Serve with: make serve"

serve: ## Serve the web frontend on :8080
	cd web && python3 -m http.server 8080

native: ## Build native helix (unchanged)
	cargo build -p helix-term

clean: ## Clean all build artifacts
	cargo clean
	rm -rf $(PKG_DIR)

grammar-clean: ## Remove compiled grammar .wasm files
	rm -rf $(GRAMMAR_OUT)

# ── Grammar compilation ──────────────────────────────────────────────────
# Compiles each tree-sitter grammar source into a shared .wasm module.
# These mirror native's .so files:
#   native:  runtime/grammars/<name>.so    (loaded via libloading)
#   wasm:    runtime/grammars/<name>.wasm  (loaded via JS WebAssembly API)
#
# Large parser.c files (>200K lines) use -O1 instead of -Os to avoid
# OOM from clang's wasm backend O(n²) behaviour on big switch tables.
LARGE_THRESHOLD := 200000

grammars: ## Compile tree-sitter grammars to .wasm shared modules
	@mkdir -p $(GRAMMAR_OUT)
	@echo "Compiling $(words $(GRAMMAR_NAMES)) grammars..."
	@ok=0; fail=0; \
	for name in $(GRAMMAR_NAMES); do \
		$(MAKE) --no-print-directory _grammar-one NAME=$$name 2>/dev/null \
			&& ok=$$((ok+1)) \
			|| fail=$$((fail+1)); \
	done; \
	echo "Grammars: $$ok built, $$fail failed"

# Internal: compile a single grammar. Called with NAME=<grammar_name>.
_grammar-one:
	@src=$(GRAMMAR_SOURCES)/$(NAME)/src; \
	out=$(GRAMMAR_OUT)/$(NAME).wasm; \
	sym=tree_sitter_$$(echo $(NAME) | tr '-' '_'); \
	objs=""; \
	parser_lines=$$(wc -l < $$src/parser.c); \
	if [ "$$parser_lines" -gt $(LARGE_THRESHOLD) ]; then \
		opt="-O1"; \
	else \
		opt="-Os"; \
	fi; \
	$(CC) --target=$(WASM_TARGET) -fPIC $$opt -nostdlib \
		-fno-builtin -fno-exceptions -fno-threadsafe-statics \
		-nostdinc -isystem $(CLANG_BUILTIN) -isystem $(SYSROOT) \
		-I $$src -c $$src/parser.c -o /tmp/_parser_$(NAME).o || exit 1; \
	objs="/tmp/_parser_$(NAME).o"; \
	if [ -f $$src/scanner.c ]; then \
		$(WASM_CC) -I $$src -c $$src/scanner.c -o /tmp/_scanner_$(NAME).o || exit 1; \
		objs="$$objs /tmp/_scanner_$(NAME).o"; \
	fi; \
	if [ -f $$src/scanner.cc ]; then \
		$(WASM_CXX) -I $$src -c $$src/scanner.cc -o /tmp/_scanner_$(NAME).o || exit 1; \
		objs="$$objs /tmp/_scanner_$(NAME).o"; \
	fi; \
	$(LD) --shared --no-entry --import-memory --allow-undefined \
		--export=$$sym $$objs -o $$out || exit 1; \
	rm -f $$objs

# Build a single grammar by name: make grammar-json, make grammar-rust, etc.
grammar-%:
	@mkdir -p $(GRAMMAR_OUT)
	$(MAKE) --no-print-directory _grammar-one NAME=$*

help: ## Show this help
	@grep -E '^[a-z_-]+:.*##' $(MAKEFILE_LIST) | awk -F ':.*## ' '{printf "  %-16s %s\n", $$1, $$2}'

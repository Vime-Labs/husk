.PHONY: all build test install check clean bump-version

all: build

build:
	cargo build --release

test:
	cargo test

check:
	cargo check

clean:
	cargo clean

## Instala o binário husk globalmente via cargo install
install:
	cargo install --path crates/husk-cli --force

## Atualiza a versão de todos os crates do workspace
## Uso: make bump-version VERSION=0.2.0
bump-version:
	@if [ -z "$(VERSION)" ]; then \
		echo "Uso: make bump-version VERSION=x.y.z"; \
		exit 1; \
	fi
	@echo "Atualizando versão para $(VERSION) em todos os crates..."
	@for toml in crates/*/Cargo.toml; do \
		sed -i "s/^version = \".*\"/version = \"$(VERSION)\"/" "$$toml"; \
		echo "  ✓ $$toml"; \
	done
	@echo ""
	@echo "Versão atualizada para $(VERSION). Execute 'make build' para compilar."
	@echo "Depois 'make install' para instalar o binário globalmente."

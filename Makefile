.PHONY: all build test install check clean bump-version

all: build

## Compila todos os crates em modo release
build:
	cargo build --release

## Executa todos os testes do workspace
test:
	cargo test

## Verifica se o código compila (mais rápido que build)
check:
	cargo check

## Limpa artefatos de compilação
clean:
	cargo clean

## Instala o binário husk globalmente (~/.cargo/bin/husk)
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

## Mostra a versão atual de cada crate
version:
	@for toml in crates/*/Cargo.toml; do \
		crate=$$(basename $$(dirname $$toml)); \
		ver=$$(grep "^version" $$toml | sed 's/version = "\(.*\)"/\1/'); \
		echo "  $$crate: $$ver"; \
	done

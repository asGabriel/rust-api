# Makefile para API Rust
.PHONY: help dev prod build build-release clean test fmt clippy migrate run-dev run-prod

# Incluir configurações dos arquivos (apenas quando necessário)

# Variáveis padrão (caso os arquivos não existam)
DEV_CONFIG_FILE := env.dev
PROD_CONFIG_FILE := env.prod

# Target padrão
help: ## Mostra esta ajuda
	@echo "🦀 Rust API - Comandos Disponíveis:"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'
	@echo ""

# Configuração de Ambiente
dev: ## 🔧 Configura variáveis para desenvolvimento
	@echo "🔧 Configurando ambiente de DESENVOLVIMENTO..."
	@if [ -f $(DEV_CONFIG_FILE) ]; then \
		echo "📋 Carregando configurações de $(DEV_CONFIG_FILE)"; \
		cat $(DEV_CONFIG_FILE); \
		echo ""; \
		echo "✅ Para aplicar as variáveis, execute:"; \
		echo "   source <(sed 's/^/export /' $(DEV_CONFIG_FILE))"; \
		echo "   # ou"; \
		echo "   set -a && source $(DEV_CONFIG_FILE) && set +a"; \
	else \
		echo "❌ Arquivo $(DEV_CONFIG_FILE) não encontrado!"; \
		echo "   Copie env.example para $(DEV_CONFIG_FILE) e ajuste as configurações."; \
	fi

prod: ## 🚀 Configura variáveis para produção
	@echo "🚀 Configurando ambiente de PRODUÇÃO..."
	@if [ -f $(PROD_CONFIG_FILE) ]; then \
		echo "📋 Carregando configurações de $(PROD_CONFIG_FILE)"; \
		cat $(PROD_CONFIG_FILE); \
		echo ""; \
		echo "✅ Para aplicar as variáveis, execute:"; \
		echo "   source <(sed 's/^/export /' $(PROD_CONFIG_FILE))"; \
		echo "   # ou"; \
		echo "   set -a && source $(PROD_CONFIG_FILE) && set +a"; \
	else \
		echo "❌ Arquivo $(PROD_CONFIG_FILE) não encontrado!"; \
		echo "   Copie env.example para $(PROD_CONFIG_FILE) e ajuste as configurações."; \
	fi

# Build Commands
build: ## 🔨 Compila em modo debug
	@echo "🔨 Compilando em modo debug..."
	cargo build

build-release: ## 🔨 Compila em modo release
	@echo "🔨 Compilando em modo release..."
	cargo build --release

# Desenvolvimento
run-dev: ## 🔧 Carrega variáveis de ambiente para desenvolvimento
	@echo "🔧 Carregando variáveis de ambiente (desenvolvimento)..."
	@if [ -f $(DEV_CONFIG_FILE) ]; then \
		echo "📋 Variáveis carregadas de $(DEV_CONFIG_FILE):"; \
		cat $(DEV_CONFIG_FILE); \
		echo ""; \
		echo "✅ Para aplicar as variáveis, execute:"; \
		echo "   export \$$(grep -v '^#' $(DEV_CONFIG_FILE) | xargs)"; \
		echo "   # ou"; \
		echo "   source <(sed 's/^/export /' $(DEV_CONFIG_FILE))"; \
	else \
		echo "❌ Arquivo $(DEV_CONFIG_FILE) não encontrado!"; \
		exit 1; \
	fi

run-prod: ## 🚀 Carrega variáveis de ambiente para produção
	@echo "🚀 Carregando variáveis de ambiente (produção)..."
	@if [ -f $(PROD_CONFIG_FILE) ]; then \
		echo "📋 Variáveis carregadas de $(PROD_CONFIG_FILE):"; \
		cat $(PROD_CONFIG_FILE); \
		echo ""; \
		echo "✅ Para aplicar as variáveis, execute:"; \
		echo "   export \$$(grep -v '^#' $(PROD_CONFIG_FILE) | xargs)"; \
		echo "   # ou"; \
		echo "   source <(sed 's/^/export /' $(PROD_CONFIG_FILE))"; \
	else \
		echo "❌ Arquivo $(PROD_CONFIG_FILE) não encontrado!"; \
		exit 1; \
	fi

# Utilitários
test: ## 🧪 Executa todos os testes
	@echo "🧪 Executando testes..."
	cargo test

test-watch: ## 👀 Executa testes em modo watch
	@echo "👀 Executando testes em modo watch..."
	cargo watch -x test

fmt: ## 📐 Formata o código
	@echo "📐 Formatando código..."
	cargo fmt

clippy: ## 🔍 Executa clippy (linting)
	@echo "🔍 Executando clippy..."
	cargo clippy -- -D warnings

check: fmt clippy test ## ✅ Executa fmt, clippy e testes

# Database
migrate: ## 🗄️ Executa migrações do banco
	@echo "🗄️ Executando migrações..."
	DATABASE_URL=$(DEV_DATABASE_URL) sqlx migrate run

migrate-revert: ## ↩️ Reverte última migração
	@echo "↩️ Revertendo última migração..."
	DATABASE_URL=$(DEV_DATABASE_URL) sqlx migrate revert

# Limpeza
clean: ## 🧹 Remove arquivos de build
	@echo "🧹 Limpando arquivos de build..."
	cargo clean

clean-all: clean ## 🧹 Limpeza completa
	@echo "🧹 Limpeza completa..."
	rm -rf target/
	rm -f Cargo.lock

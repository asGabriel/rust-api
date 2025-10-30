# Makefile para API Rust
.PHONY: help dev prod build build-release clean test fmt clippy migrate run-dev run-prod

# Variáveis padrão
DEV_CONFIG_FILE := env.dev
PROD_CONFIG_FILE := env.prod

env-dev: ## Executa a aplicação em modo desenvolvimento carregando variáveis de env.dev
	@echo "🚀 Iniciando em modo desenvolvimento..."
	@export $$(grep -v '^#' $(DEV_CONFIG_FILE) | grep -v '^$$' | xargs)

env-prod: ## Executa a aplicação em modo produção carregando variáveis de env.prod
	@echo "🚀 Iniciando em modo produção..."
	@export $$(grep -v '^#' $(PROD_CONFIG_FILE) | grep -v '^$$' | xargs)

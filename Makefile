# Makefile para API Rust
.PHONY: help dev prod build build-release clean test fmt clippy migrate run-dev run-prod

# Variáveis padrão
DEV_CONFIG_FILE := env.dev
PROD_CONFIG_FILE := env.prod

env-dev: ## Carrega variáveis de env.dev no shell atual (use: eval $$(make env-dev))
	@grep -v '^#' $(DEV_CONFIG_FILE) | grep -v '^$$' | while IFS='=' read -r key value; do \
		value=$${value#\"}; value=$${value%\"}; \
		echo "export $$key=$$value"; \
	done

env-prod: ## Carrega variáveis de env.prod no shell atual (use: eval $$(make env-prod))
	@grep -v '^#' $(PROD_CONFIG_FILE) | grep -v '^$$' | while IFS='=' read -r key value; do \
		value=$${value#\"}; value=$${value%\"}; \
		echo "export $$key=$$value"; \
	done

echo-env:
	@echo "TELEGRAM_API_URL=$(TELEGRAM_API_URL)"

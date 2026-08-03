# rust-api

Workspace Cargo com múltiplos serviços (crates binários) e libs compartilhadas. Cada serviço é um processo Axum independente, rodando na mesma instância Postgres.

## Estrutura

```
rust-api/
  api/            # serviço principal: finance manager (debt, income, payment, invoice, financial_instrument) + auth (JWT/bcrypt)
  matchmaking/     # serviço novo, ainda só scaffold (AppState + GET /api/status)
  lib/
    database/      # DbPool (sqlx Postgres), macro push_filter! para query builder
    http-error/     # HttpError/HttpResult padronizado (RFC 7807), features: http/axum/sqlx/reqwest
    util/           # helpers de data, deleted_by, etc.
    telegram_api/    # client para a Telegram API (usado só pelo api)
  migrations/
    <serviço>/      # migrations do serviço, uma subpasta por serviço (ex: migrations/api, migrations/matchmaking)
```

Cada serviço tem seu próprio `main.rs`, sua própria porta e seu próprio `AppState`. Não há comunicação HTTP entre `api` e `matchmaking` hoje — são independentes, compartilhando apenas o Postgres e as libs de `lib/*`.

## Serviços

Contexto de negócio e regras específicas de cada serviço (complementa a seção de Estrutura acima).

### matchmaking

Responsável por organizar sorteio de equipes para partidas de esportes.

Caso de uso atual: sorteio para um grupo de vôlei (jogadores, sessões, partidas). As regras de sorteio (critérios de balanceamento de times, restrições, etc.) ainda serão definidas e documentadas aqui conforme forem implementadas nos próximos PRs.

## Comandos

```bash
# build de um serviço específico (nome do crate, ver Cargo.toml de cada um)
cargo build -p <crate>

# build de tudo (o mesmo que a CI roda)
cargo build --all-features

# rodar localmente (precisa de .env com DATABASE_URL etc — ver env.example)
cargo run -p <crate>   # cada serviço lê sua própria porta do .env (ver tabela de env vars)

# testes (poucos hoje, principalmente em lib/util)
cargo test --all-features

# lint/format (sem config customizada, usa defaults do rustfmt/clippy)
cargo fmt
cargo clippy --all-features

# migrations: uma pasta por serviço em migrations/<serviço> (sqlx-cli, contra DATABASE_URL)
sqlx migrate run --source migrations/<serviço>
sqlx migrate revert --source migrations/<serviço>
sqlx migrate add --source migrations/<serviço> nome_da_migration
```

Para rodar vários serviços juntos localmente, sobe cada um em um terminal separado — todos compartilham o mesmo `.env`, cada um só lê sua própria porta.

## Convenções de módulo (valem para qualquer serviço do workspace)

Cada módulo/domínio segue o padrão `domain / handler / repository / routes`:

- **`domain/`** — structs do modelo (ex: `User`, `Debt`), sem lógica de infra.
- **`repository/`** — trait `DynXRepository` + impl (`XRepositoryImpl`) que recebe `&Pool<Postgres>` e faz as queries via `sqlx`.
- **`handler/`** — trait `XHandler` (`#[async_trait]`) + impl (`XHandlerImpl`) que orquestra regra de negócio, chamando o repository. É o que fica dentro do `AppState`, sempre atrás de `Arc<...>`.
- **`routes/`** — funções `async fn` que recebem `State<AppState>` + `Json`/`Path`/`HeaderMap`, chamam o handler e retornam `HttpResult<impl IntoResponse>`. `configure_routes()`/`configure_service_routes()` monta o `Router<AppState>` do módulo.

Tudo isso é montado/wireado no `main.rs` de cada serviço (constrói repository → handler → `AppState`), nunca dentro dos módulos.

**Novo crate vs. novo módulo:** crie um crate novo quando for um serviço com ciclo de vida/deploy próprio (porta própria, pode escalar ou reiniciar independente). Crie um módulo novo dentro de um crate existente quando for só mais uma feature do mesmo bounded context (ex: uma nova entidade dentro de um domínio já existente).

## Variáveis de ambiente

Ver `env.example` para a lista completa. Resumo por serviço:

| Variável | Usado por | Observação |
|---|---|---|
| `DATABASE_URL` | api, matchmaking | mesma instância Postgres para os dois serviços |
| `PORT` | api | default 8080 |
| `MATCHMAKING_PORT` | matchmaking | default 8081 |
| `JWT_SECRET` | api | usado só pelo módulo `auth`; `matchmaking` ainda não valida token |
| `TELEGRAM_API_URL` / `TELEGRAM_API_TOKEN` | api | usado via lib `telegram_api` |

## Git workflow

- Branches: `feat/<nome>`, `fix/<nome>`.
- PRs mergeados em `main` via GitHub.
- **Commits não devem ter trailer `Co-Authored-By` nem referência de sessão do Claude** — o autor no GitHub deve aparecer só como o usuário (gabriel).

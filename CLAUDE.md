# rust-api

Workspace Cargo com um serviço web (`api`) e libs compartilhadas. Hoje só há infraestrutura disponível para rodar **uma** instância de webservice, então todo domínio de negócio (finance manager, auth, matchmaking) vive como módulo dentro do crate `api`, em vez de crates/processos separados. Se essa restrição de infra mudar, algum módulo pode ser extraído para um crate próprio (ver "Novo crate vs. novo módulo" abaixo).

## Estrutura

```
rust-api/
  api/            # único serviço Axum: módulos auth (JWT/bcrypt), finance_manager (debt, income, payment, invoice, financial_instrument) e matchmaking
  lib/
    database/      # DbPool (sqlx Postgres), macro push_filter! para query builder
    http-error/     # HttpError/HttpResult padronizado (RFC 7807), features: http/axum/sqlx/reqwest
    util/           # helpers de data, deleted_by, etc.
    telegram_api/    # client para a Telegram API
  migrations/
    <domínio>/      # migrations agrupadas por domínio/schema (ex: migrations/api, migrations/matchmaking), todas contra o mesmo Postgres do serviço único
```

Todos os módulos rodam no mesmo processo (`api/src/main.rs`), compartilhando o mesmo `AppState`, pool de conexão e porta (`PORT`).

## Módulos de negócio

Contexto de negócio e regras específicas de cada módulo (complementa a seção de Estrutura acima).

### matchmaking

Responsável por organizar sorteio de equipes para partidas de esportes. Hoje roda como módulo dentro do serviço `api` (não como processo separado), pela limitação de infraestrutura descrita no topo deste arquivo.

Caso de uso atual: sorteio para um grupo de vôlei de praia (beach volley) — jogadores (`Player`), dias de jogo (`Session`, com configurações padrão e quadras disponíveis), duplas (`Team`) e partidas registradas (`Match`). Repositórios são persistidos em Postgres (schema `matchmaking`, migrations em `migrations/matchmaking`), no mesmo padrão dos demais módulos. As regras de sorteio (critérios de balanceamento de times, restrições, etc.) ainda serão definidas e documentadas aqui conforme forem implementadas nos próximos PRs.

## Comandos

```bash
# build (nome do crate, ver Cargo.toml de cada um)
cargo build -p api

# build de tudo (o mesmo que a CI roda)
cargo build --all-features

# rodar localmente (precisa de .env com DATABASE_URL, PORT, JWT_SECRET etc — ver env.example)
cargo run -p api

# testes (poucos hoje, principalmente em lib/util)
cargo test --all-features

# lint/format (sem config customizada, usa defaults do rustfmt/clippy)
cargo fmt
cargo clippy --all-features

# migrations: uma pasta por domínio em migrations/<domínio>, todas contra o mesmo DATABASE_URL
sqlx migrate run --source migrations/<domínio>
sqlx migrate revert --source migrations/<domínio>
sqlx migrate add --source migrations/<domínio> nome_da_migration
```

## Convenções de módulo (valem para qualquer módulo/crate do workspace)

Cada módulo/domínio segue o padrão `domain / handler / repository / routes`:

- **`domain/`** — structs do modelo (ex: `User`, `Debt`), sem lógica de infra. Condições booleanas usadas para validação/regra de negócio (ex: "esse `winner_team_id` é um dos times do `Match`?") devem ser extraídas como método nomeado na própria struct (ex: `Match::has_team`, `Match::is_finished`), não deixadas como expressão inline dentro de outro método. Quando a validação envolver múltiplos passos ou precisar de um valor de escopo (ex: `session_id`), usar uma struct validadora dedicada com esse valor vinculado na construção (ex: `TeamValidator::new(session_id)`) em vez de funções soltas no módulo.
- **`repository/`** — trait `DynXRepository` + impl (`XRepositoryImpl`) que recebe `&Pool<Postgres>` e faz as queries via `sqlx`.
- **`handler/`** — trait `XHandler` (`#[async_trait]`) + impl (`XHandlerImpl`) que orquestra regra de negócio, chamando o repository. É o que fica dentro do `AppState`, sempre atrás de `Arc<...>`.
- **`routes/`** — funções `async fn` que recebem `State<AppState>` + `Json`/`Path`/`HeaderMap`, chamam o handler e retornam `HttpResult<impl IntoResponse>`. `configure_routes()`/`configure_service_routes()` monta o `Router<AppState>` do módulo.

Tudo isso é montado/wireado no `main.rs` do crate (constrói repository → handler → `AppState`), nunca dentro dos módulos.

**Novo crate vs. novo módulo:** por padrão, novo domínio de negócio = novo módulo dentro do crate `api` — é a única opção viável hoje, já que só há infraestrutura para uma instância de webservice (ver topo deste arquivo). Só vale extrair um módulo para um crate/processo próprio quando: (1) houver infra disponível para rodar mais de uma instância, e (2) o domínio realmente se beneficiar de ciclo de vida/deploy/escala independente. Fora isso, tudo é módulo dentro de `api`.

## Variáveis de ambiente

Ver `env.example` para a lista completa.

| Variável | Observação |
|---|---|
| `DATABASE_URL` | Postgres compartilhado por todos os módulos |
| `PORT` | porta do serviço, default 8080 |
| `JWT_SECRET` | usado pelo módulo `auth`; `matchmaking` ainda não valida token |
| `TELEGRAM_API_URL` / `TELEGRAM_API_TOKEN` | usado via lib `telegram_api` |

## Git workflow

- Branches: `feat/<nome>`, `fix/<nome>`.
- PRs mergeados em `main` via GitHub.
- **Commits não devem ter trailer `Co-Authored-By` nem referência de sessão do Claude** — o autor no GitHub deve aparecer só como o usuário (gabriel).

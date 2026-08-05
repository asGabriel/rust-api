---
name: gb-matchmaking-business-rules
description: Regras de negócio do módulo de matchmaking — critérios de pareamento, restrições, prioridades e casos-limite.
---

# Regras de negócio — matchmaking

> Módulo ainda no início da implementação. Este documento é um esqueleto: as
> seções abaixo devem ser preenchidas manualmente conforme as regras forem
> sendo decididas e implementadas em `api/src/modules/matchmaking`. Não
> inferir regras a partir do código atual — só documentar aqui o que for
> explicitamente decidido como regra de negócio.

## Critérios de pareamento

- Toda `Session` tem um `GameMode` (`Male`, `Female` ou `Mixed`), que filtra
  quem pode formar dupla junto: `Male`/`Female` só pareiam jogadores do
  mesmo gênero; `Mixed` forma cada `Team` com metade dos jogadores homens e
  metade mulheres (com `players_per_team = 2`, na prática 1 homem + 1
  mulher).
- O primeiro sorteio de `Team`s de uma `Session` é aleatório, sem nenhum
  critério de balanceamento (nível, histórico de parceria, etc.) — v1
  deliberadamente simples, pois no início da sessão não há histórico para
  balancear. Sorteios futuros podem somar outros critérios em cima do
  `GameMode`.
- Implementado por `TeamDrawer::draw` (`api/src/modules/matchmaking/domain/team.rs`),
  chamado por `TeamHandlerImpl::draw_teams` via `POST
  /matchmaking/teams/{session_id}/draw`.

<!--
Como jogadores são agrupados em duplas/times (Team) e como partidas
(Match) são formadas a partir das duplas de uma Session. Ex: nível de
habilidade, histórico de parceria, aleatoriedade controlada, etc.
-->

## Restrições

- Um jogador não pode aparecer em duas `Team`s da mesma `Session`.
- Um jogador não pode se repetir dentro da mesma `Team`.
- `GameMode::Mixed` exige `players_per_team` par (para dividir metade
  homens / metade mulheres por `Team`). Validado tanto na criação/edição da
  `Session` (`GameMode::validate_players_per_team`, chamado por
  `Session::new`/`set_settings`/`set_game_mode`) quanto no sorteio
  (`TeamDrawer::draw`), para que uma `Session` nunca fique salva numa
  configuração que o sorteio não consegue honrar.
- `draw_teams` só pode ser chamado uma vez por `Session` (é o sorteio de
  *inicialização*): se a `Session` já tiver alguma `Team`, retorna
  `HttpError::conflict`. Não existe hoje endpoint para resetar/re-sortear.
- Um `Match` não pode ter as duas equipes iguais (`team_a_id != team_b_id`).
- O resultado de um `Match` só pode ser reportado uma vez: reportar de novo
  um `Match` que já tem `winner_team_id` retorna `HttpError::conflict`.
- `winner_team_id` reportado precisa ser `team_a_id` ou `team_b_id` do
  próprio `Match`; qualquer outro valor retorna `HttpError::bad_request`.

Validado por `Match::new`/`Match::finish`
(`api/src/modules/matchmaking/domain/matches.rs`), chamados por
`MatchHandlerImpl::create_match`/`report_match_result` via `POST
/matchmaking/matches/` e `POST /matchmaking/matches/{match_id}/result`.

Restrições de duplicidade de jogador validadas por
`TeamValidator::validate_new_team` (`api/src/modules/matchmaking/domain/team.rs`),
chamado tanto por `TeamHandlerImpl::create_team` quanto por
`TeamHandlerImpl::draw_teams`.

<!--
Condições que uma implementação NUNCA pode violar. Ex: número mínimo/máximo
de jogadores por Session, quadras disponíveis (available_courts) não podem
ser excedidas, etc.
-->

## Prioridades

_A definir._

<!--
Quando múltiplos critérios de pareamento entram em conflito, qual prevalece.
Ex: balanceamento de nível tem prioridade sobre variar parceiros.
-->

## Casos-limite conhecidos

- Jogadores que sobram sem completar um time cheio (ou, no modo `Mixed`, do
  gênero que já se esgotou) ficam de fora do sorteio silenciosamente —
  `draw_teams` retorna só os times formados, sem indicar quem ficou de fora.
  Ainda não decidido se isso deve mudar (ex: devolver lista de não
  alocados).
- `draw_teams` faz o check de "sessão já tem times" e os inserts em
  chamadas separadas ao repositório — duas chamadas concorrentes para a
  mesma `Session` podem, em teoria, passar pelo check antes de qualquer
  insert acontecer (TOCTOU). Não tratado ainda; aceitável hoje por ser
  repositório em memória de processo único, sem carga concorrente real.

<!--
Situações especiais já discutidas/decididas. Ex: número ímpar de jogadores,
Session sem jogadores suficientes para as quadras disponíveis, jogador
removido de uma Session após os times já terem sido sorteados, etc.
-->

## Casos-limite conhecidos (continuação)

- Ainda não existe checagem de que `team_a_id`/`team_b_id` de um `Match`
  pertencem de fato a `Team`s da `Session` informada, nem de que uma
  `Team`/quadra não tenha mais de um `Match` em andamento simultaneamente.
  Estrutura inicial deliberadamente simples (só o estado
  em-andamento → resultado reportado); regras de "o que sortear a seguir
  quando uma partida termina" ainda serão definidas e documentadas aqui
  conforme forem implementadas.

## Histórico de mudanças

- 2026-08-04 — `Match` passa a nascer sem resultado (`winner_team_id`/
  `played_at` como `Option`, partida "em andamento" na quadra) via `POST
  /matchmaking/matches/`; novo endpoint `POST
  /matchmaking/matches/{match_id}/result` reporta o resultado
  (`ReportMatchResultRequest { winner_team_id }`), validando que a partida
  ainda não tem resultado e que o vencedor é uma das duas equipes do
  `Match`. Primeira peça da estrutura de "regras de sorteio conforme os
  jogos vão finalizando" — regras de qual será o próximo sorteio após um
  resultado ainda serão decididas e adicionadas aqui.
- 2026-08-04 — implementada validação em `create_team`: rejeita jogador
  duplicado dentro da mesma `Team` (`HttpError::bad_request`) e jogador já
  presente em outra `Team` da mesma `Session` (`HttpError::conflict`).
- 2026-08-04 — adicionado `GameMode` (`Male`/`Female`/`Mixed`) como campo
  obrigatório de `Session`, e rota `POST /matchmaking/teams/{session_id}/draw`
  para o sorteio inicial (aleatório) de `Team`s a partir dos jogadores
  confirmados na `Session`, respeitando o `GameMode`.

<!--
Registro cronológico de decisões de regra de negócio, com data e motivo.
Ex: "2026-08-04 — decidido que sorteio de duplas é aleatório sem
balanceamento de nível na v1, ver discussão em <link/PR>."
-->

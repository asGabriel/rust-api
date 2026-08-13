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
- O sorteio (tanto o inicial quanto os que acontecem conforme partidas
  terminam) evita, best-effort, formar uma `Team` com dois jogadores que já
  jogaram juntos como parceiros na mesma `Session` — mas aceita repetir se
  não houver alternativa (nunca trava o sorteio por causa disso). O
  histórico usado é só de `Team`s que de fato entraram em algum `Match`;
  uma dupla que só passou pela fila sem chegar a jogar não conta.
  Implementado por `PartnerHistory` + o algoritmo greedy
  most-constrained-first de `TeamDrawer::draw`
  (`api/src/modules/matchmaking/domain/team_drawer.rs`).
- Jogadores que sobram (não fecham um time cheio, ou, no modo `Mixed`, são
  do gênero já esgotado) nunca são descartados: formam uma `Team`
  incompleta, visível via `GET /matchmaking/teams/{session_id}`, esperando
  outro jogador ser liberado para completá-la.
- Implementado por `TeamDrawer::draw` (sorteio inicial, sem histórico) e por
  `TeamHandlerImpl::draw_teams` via `POST
  /matchmaking/teams/{session_id}/draw`.

<!--
Como jogadores são agrupados em duplas/times (Team) e como partidas
(Match) são formadas a partir das duplas de uma Session. Ex: nível de
habilidade, histórico de parceria, aleatoriedade controlada, etc.
-->

### Fila e rotação de quadra

- Toda `Session` tem um `ShuffleType`, que escolhe a estratégia de rotação
  usada conforme as partidas terminam. Hoje só existe a estratégia
  `KingAndQueen` (a fila contínua com rotação por vitórias descrita abaixo),
  mas o campo já é explícito e obrigatório na `Session` (mesmo padrão do
  `GameMode`) para permitir uma estratégia diferente no futuro sem que
  nenhuma `Session` mude de comportamento silenciosamente.
- Uma `Team` tem um `status`: `Waiting` (na fila, disponível pra entrar em
  quadra), `Holding` (venceu e está segurando a quadra aguardando o próximo
  desafiante) ou `Disbanded` (perdeu, ou girou pra fora por ter batido o
  cap de vitórias — jogadores livres, registro mantido só como histórico de
  parceria).
- **Vencedor:** ao vencer, a `Team` fica segurando a quadra e joga mais uma
  (`Team::register_win`, `status = Holding`). Se vencer essa também (2
  vitórias seguidas na mesma quadra — `MAX_CONSECUTIVE_WINS`), ela também é
  desfeita: seus jogadores voltam pra fila igual a um time perdedor, e a
  quadra precisa de duas `Team`s novas da fila.
- **Perdedor:** sempre se desfaz (`Team::disband`); seus jogadores voltam
  pra fila.
- **Fila (FIFO):** jogadores liberados (do perdedor, e do vencedor quando
  bate o cap) são inseridos na fila em ordem aleatória — cada um tenta
  completar uma `Team` incompleta compatível (mesmo gênero necessário, no
  caso `Mixed`) já esperando na fila; se não achar, inicia uma nova `Team`
  incompleta. Sem regra especial de "qual dos jogadores liberados" completa
  a sobra existente. Implementado por `TeamQueueManager::release_players`
  (`api/src/modules/matchmaking/domain/team_queue.rs`).
- **Continuação automática da quadra:** ao reportar o resultado de um
  `Match` (`POST /matchmaking/matches/{match_id}/result`), o sistema já
  cria automaticamente o próximo `Match` daquela quadra — vencedor (se
  ainda segurando) contra o primeiro time completo da fila
  (`TeamQueueManager::next_complete_teams`), ou duas `Team`s novas da fila
  se o vencedor também girou. Não é preciso chamar `POST
  /matchmaking/matches/` de novo pra continuar uma quadra já ocupada — esse
  endpoint só serve pra abrir uma quadra pela primeira vez. Se a fila ainda
  não tiver `Team`s completas suficientes, a quadra fica ociosa até ter
  (não é tratado como erro).
- Orquestrado por `TeamHandlerImpl::resolve_match_result`
  (`api/src/modules/matchmaking/handler/team.rs`), chamado por
  `MatchHandlerImpl::report_match_result`
  (`api/src/modules/matchmaking/handler/matches.rs`).

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
  `HttpError::conflict`. Não existe hoje endpoint para resetar/re-sortear
  do zero (a fila segue evoluindo sozinha depois, via resultado de partida).
- Um `Match` não pode ter as duas equipes iguais (`team_a_id != team_b_id`).
- O resultado de um `Match` só pode ser reportado uma vez: reportar de novo
  um `Match` que já tem `winner_team_id` retorna `HttpError::conflict`.
- `winner_team_id` reportado precisa ser `team_a_id` ou `team_b_id` do
  próprio `Match`; qualquer outro valor retorna `HttpError::bad_request`.
- Para iniciar um `Match` (`POST /matchmaking/matches/`), as duas `Team`s
  precisam: pertencer à `Session` informada, estar completas
  (`Team::is_complete`, não uma sobra esperando parceiro), não estar
  `Disbanded`, e não estar já disputando outro `Match` em andamento em
  outra quadra (`Match::busy_team_ids`). Validado por
  `MatchStartValidator::validate_start`.

Validado por `Match::new`/`Match::finish`/`MatchStartValidator`
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

- `draw_teams` faz o check de "sessão já tem times" e os inserts em
  chamadas separadas ao repositório — duas chamadas concorrentes para a
  mesma `Session` podem, em teoria, passar pelo check antes de qualquer
  insert acontecer (TOCTOU). Não tratado ainda; aceitável hoje por ser
  repositório em memória de processo único, sem carga concorrente real. O
  mesmo vale, em tese, para `resolve_match_result`/`create_match`
  concorrentes na mesma quadra.
- Resolvido nesta rodada (ver Histórico de mudanças): jogadores que
  sobravam no sorteio eram descartados silenciosamente; não havia checagem
  de `Team` pertencente à `Session` nem de partida simultânea na mesma
  `Team`/quadra; não existia noção do que acontece depois que um `Match`
  termina.
- Em `Mixed` mode, uma `Team` incompleta esperando um gênero específico só
  é completada por um jogador liberado desse mesmo gênero
  (`TeamQueueManager::needs_gender`) — se o desbalanceamento de gênero for
  grande, podem se acumular várias `Team`s incompletas em paralelo (uma por
  gênero em falta), não só uma. Comportamento aceito, não é bug.

<!--
Situações especiais já discutidas/decididas. Ex: número ímpar de jogadores,
Session sem jogadores suficientes para as quadras disponíveis, jogador
removido de uma Session após os times já terem sido sorteados, etc.
-->

## Histórico de mudanças

- 2026-08-12 — adicionado `ShuffleType` (hoje só `KingAndQueen`) como campo
  obrigatório de `Session`. Formaliza como estratégia nomeada e selecionável
  a rotação de fila contínua por vitórias que já existia (2026-08-07),
  sem mudar seu comportamento — deixa de ser implícita/hardcoded.
- 2026-08-07 — Fila contínua de duplas com rotação por vitórias: `Team`
  ganha `status` (`Waiting`/`Holding`/`Disbanded`) e `consecutive_wins`.
  Perdedor de um `Match` sempre é desfeito; vencedor segura a quadra até 2
  vitórias seguidas, depois também é desfeito. Jogadores liberados entram
  numa fila FIFO (`TeamQueueManager`) que nunca descarta sobra (formam
  `Team`s incompletas) e evita, best-effort, repetir parceiros que já
  jogaram juntos na `Session` (`PartnerHistory`, olhando só `Team`s que de
  fato jogaram). `report_match_result` passa a criar automaticamente o
  próximo `Match` da quadra que ficou livre, sem precisar de nova chamada
  manual a `create_match`. `create_match` (usado só pra abrir uma quadra
  pela primeira vez) passa a validar via `MatchStartValidator` que as duas
  `Team`s pertencem à `Session`, estão completas, não estão `Disbanded` e
  não estão ocupadas em outro `Match` em andamento.
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

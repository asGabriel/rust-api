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

- Toda `Session` tem um `GameMode` (`Male`, `Female`, `Mixed` ou `Open`), que
  filtra quem pode formar dupla junto: `Male`/`Female` só pareiam jogadores
  do mesmo gênero; `Mixed` forma cada `Team` com metade dos jogadores homens
  e metade mulheres (com `players_per_team = 2`, na prática 1 homem + 1
  mulher); `Open` ignora gênero totalmente — qualquer jogador pode formar
  dupla com qualquer outro.
- *Quem* entra no time a cada rodada é decidido pela lista de jogadores
  (`session_queue`) — ver "Fila e rotação de quadra". `next_challenger` pega
  os primeiros da lista sem tentar alternativas de pareamento; só **anota**
  (`repeats_partner`, nunca bloqueia) se os escolhidos já jogaram juntos na
  mesma `Session`, via `PartnerHistory` (só conta `Team`s que de fato
  entraram em algum `Match`). O `TeamDrawer::draw` só é usado no **seeding**
  inicial (`queue/seed`), pra distribuir os jogadores de abertura entre as
  quadras — nesse ponto o histórico está vazio, então é só embaralhamento
  respeitando gênero.
- Quem não entra num time agora (não é dos primeiros da lista, ou, em
  `Mixed`, é do gênero já esgotado na rodada) continua na `session_queue`,
  visível via `GET /matchmaking/sessions/{id}/queue`, e entra numa rodada
  seguinte conforme partidas terminam.

<!--
Como jogadores são agrupados em duplas/times (Team) e como partidas
(Match) são formadas a partir das duplas de uma Session. Ex: nível de
habilidade, histórico de parceria, aleatoriedade controlada, etc.
-->

### Fila e rotação de quadra

> Reescrito em 2026-08-30 (ver "Histórico de mudanças"): a fila deixou de
> ser uma fila de `Team`s pré-formadas e passou a ser uma **lista de
> jogadores individuais** por `Session` (`matchmaking.session_queue`). Times
> só são formados no momento de entrar em quadra, como *sugestão* que o
> operador confirma. Removeu `TeamQueueManager`/`release_players` e o
> conceito de `Team` `Waiting`.

#### A lista (`session_queue`)

- Uma linha por jogador da `Session` que **não** está em quadra nem
  `Holding`. Colunas: `player_id`, `games_played` (nº de `Match`es que o
  jogador já terminou — mantido na escrita, não derivado), `enqueued_at`
  (vira `now()` toda vez que o jogador (re)entra na lista) e `pinned` /
  `pinned_at` (prioridade manual).
- **Ordem de quem entra primeiro:**
  `pinned DESC, pinned_at ASC, games_played ASC, enqueued_at ASC`.
  A justiça é por **jogos disputados** (quem jogou menos entra antes),
  com tempo de espera como desempate.
- "Ninguém volta na hora" é consequência da ordem, não uma regra à parte:
  quem sai de uma partida entra na lista com `games_played + 1` **e**
  `enqueued_at = now()` — afunda nos dois critérios.
- Ao criar a `Session`, todos os `player_ids` confirmados entram na lista
  com `games_played = 0`.

#### `Team` — status

- `Draft` — sugestão de time formada pra entrar numa quadra, ainda **não**
  em `Match`; editável (`PATCH /matchmaking/teams/{id}/players`) e
  descartável (`DELETE`). Seus jogadores já saíram da `session_queue`
  (reservados).
- `Playing` — em `Match` aberto (derivado por trigger, como antes).
- `Holding` — venceu e está segurando a quadra.
- `Disbanded` — perdeu, girou por bater o cap de vitórias, ou `Draft`
  descartado. Mantido só como histórico de parceria.

#### Ao reportar o resultado (`POST /matchmaking/matches/{match_id}/result`)

1. `Match` é finalizado (`Match::finish`).
2. **Perdedor:** `Team::disband`; cada jogador volta pra `session_queue`
   (`games_played + 1`, `enqueued_at = now()`).
3. **Vencedor:** `Team::register_win`. Se foi a 2ª vitória seguida na
   mesma quadra (`MAX_CONSECUTIVE_WINS`), também é desfeito e seus
   jogadores voltam pra lista (`games_played + 1`); senão fica `Holding`.
4. **Preenche as quadras ociosas.** Varre toda quadra que a `Session` já
   abriu (última `Match` de cada `court`); pra cada uma cuja última partida
   já tem resultado: `needed` = 1 se há `Team` `Holding` nela, senão 2.
   Chama `next_challenger` `needed` vezes, cada chamada **removendo da
   `session_queue`** os jogadores escolhidos (mesma transação) e criando um
   `Team` `Draft`. Ordem: quadra ociosa há mais tempo primeiro.
5. A resposta traz, por quadra, o(s) `draft_team_id`(s) e o
   `holding_team_id` — **não inicia `Match` nenhum**.

Orquestrado por `TeamHandlerImpl::resolve_match_result`, chamado por
`MatchHandlerImpl::report_match_result`.

#### `next_challenger` (sugestão automática)

- Pega da lista, **na ordem**, os primeiros jogadores que satisfazem a
  composição de gênero do `GameMode`: `Mixed` = `players_per_team / 2` de
  cada gênero (cada gênero na sua própria ordem); `Male`/`Female`/`Open` =
  os `players_per_team` primeiros.
- Se não há jogadores suficientes do gênero necessário → retorna `None`,
  **não forma nada** pra aquela quadra (fica ociosa, a resposta sinaliza).
  O operador pode montar a dupla manualmente (ver "Montagem manual").
- Pega exatamente os N do topo, na ordem da lista — **sem** `TeamDrawer`,
  sem tentar alternativas pra evitar repetição. Só marca `repeats_partner`
  (via `PartnerHistory`) se os escolhidos já jogaram juntos na `Session`;
  é dica pro operador, não bloqueio — se repetir e ele não gostar, troca no
  `Draft`. Retorna `ChallengerSuggestion { player_ids, repeats_partner }`.
  Implementado em `SessionQueue::next_challenger`
  (`api/src/modules/matchmaking/domain/queue.rs`).

#### Confirmar / descartar / editar o `Draft`

- **Confirmar:** `POST /matchmaking/courts/{court}/start` com os dois
  `team_id`s → valida e cria o `Match` (drafts viram `Playing`).
- **Descartar:** `DELETE /matchmaking/teams/{draft_id}` → jogadores voltam
  pra `session_queue` (`enqueued_at = now()`, `games_played` inalterado).
- **Editar roster:** `PATCH /matchmaking/teams/{draft_id}/players` — trocar
  jogadores antes de confirmar. Quem entra sai da `session_queue`; quem sai
  volta pra ela.

#### Montagem manual (contingência) vs. automático

- O caminho **automático** (`next_challenger`, seeding) **respeita o
  `GameMode`**: não monta dupla fora da composição de gênero.
- O caminho **manual** (criar `Draft` direto, ou editar o roster de um
  `Draft`) **não** valida gênero — o operador pode compor qualquer dupla
  (ex.: 2 homens numa `Session` `Mixed`). Mesmo princípio do antigo
  `create_team`: a montagem manual é soberana sobre a config da `Session`.
- **Iniciar o `Match`** valida, pra os dois times: jogadores confirmados na
  `Session`, ninguém já `Playing`/`Holding` em outra quadra, e **roster com
  exatamente `players_per_team` jogadores** — mas **não** valida gênero.

#### Prioridade manual

- Em vez de "time prioritário", o operador **fixa jogadores** no topo da
  lista: `pinned = true` (ver ordenação acima). O próximo `next_challenger`
  vai pegar esses jogadores primeiro. Não há mais `Team::with_priority` nem
  `create_priority_team`.

#### Seeding inicial

- `POST /matchmaking/sessions/{id}/queue/seed` forma as partidas de
  abertura: chama `next_challenger` até `2 * available_courts` vezes (ou o
  que a lista permitir) e devolve os `Draft`s — mesmo fluxo "formar →
  revisar → confirmar". Substitui `draw_teams`.
- **Trade-off aceito (preenchimento guloso):** quadras ociosas são
  preenchidas por ordem de espera, não maximizando o nº de quadras ativas —
  uma quadra que precisa de 2 times pode esvaziar a lista antes de uma
  quadra que precisava de só 1. Prioriza justiça sobre utilização.

## Restrições

- Um jogador não pode aparecer em duas `Team`s da mesma `Session`.
- Um jogador não pode se repetir dentro da mesma `Team`.
- `GameMode::Mixed` exige `players_per_team` par (para dividir metade
  homens / metade mulheres por `Team`). Validado na criação/edição da
  `Session` (`GameMode::validate_players_per_team`, chamado por
  `Session::new`/`set_settings`/`set_game_mode`) e honrado pelo caminho
  automático (`SessionQueue::next_challenger` e o seeding). A montagem
  manual pode ignorar (ver "Montagem manual" em "Fila e rotação de quadra").
- O seeding (`POST /matchmaking/sessions/{id}/queue/seed`) só forma
  partidas de abertura enquanto a `Session` não tiver nenhum `Match`; depois
  disso a rotação segue por resultado de partida. Não há re-seed.
- Um `Match` não pode ter as duas equipes iguais (`team_a_id != team_b_id`).
- O resultado de um `Match` só pode ser reportado uma vez: reportar de novo
  um `Match` que já tem `winner_team_id` retorna `HttpError::conflict`.
- `winner_team_id` reportado precisa ser `team_a_id` ou `team_b_id` do
  próprio `Match`; qualquer outro valor retorna `HttpError::bad_request`.
- Para iniciar um `Match` (`POST /matchmaking/courts/{court}/start`), os
  dois times precisam: pertencer à `Session` informada, ter **exatamente
  `players_per_team` jogadores** no roster, não estar `Disbanded`, e nenhum
  jogador seu estar já `Playing`/`Holding` em outra quadra
  (`Match::busy_team_ids`). Gênero **não** é validado aqui. Validado por
  `MatchStartValidator::validate_start`.

Validado por `Match::new`/`Match::finish`/`MatchStartValidator`
(`api/src/modules/matchmaking/domain/matches.rs`), chamados por
`MatchHandlerImpl::create_match`/`report_match_result` via `POST
/matchmaking/matches/` e `POST /matchmaking/matches/{match_id}/result`.

- `POST /matchmaking/teams/` (`create_team`) é a via manual de montar um
  `Team` `Draft`: independente do `GameMode` da `Session` (que só restringe
  o caminho automático, nunca a montagem manual), permite escolher
  jogadores específicos — contingência quando a sugestão automática não
  serve. Exige que todo `player_id` esteja confirmado em
  `Session::player_ids` e que nenhum já esteja `Playing`/`Holding` (ou num
  outro `Draft`) da mesma `Session`; jogadores de um `Team` `Disbanded` já
  estão livres e não bloqueiam. Os jogadores escolhidos saem da
  `session_queue`.

Restrições de duplicidade/elegibilidade de jogador validadas por
`TeamValidator::validate_new_team` (`api/src/modules/matchmaking/domain/team.rs`).

<!--
Condições que uma implementação NUNCA pode violar. Ex: número mínimo/máximo
de jogadores por Session, quadras disponíveis (available_courts) não podem
ser excedidas, etc.
-->

## Prioridades

- Prioridade é por **jogador**, não por time: `pinned = true` numa linha da
  `session_queue` (`PATCH /matchmaking/sessions/{id}/queue/{player_id}`).
  Jogadores `pinned` vêm antes de todos os outros na ordem da lista
  (`pinned DESC, pinned_at ASC, …`), então o próximo `next_challenger` os
  pega primeiro — é o próximo desafiante garantido assim que uma quadra
  ficar livre (não necessariamente *essa* quadra, se mais de uma abrir ao
  mesmo tempo).
- `pinned` é limpo quando o jogador entra numa partida (sai da lista) e não
  volta automaticamente — se o operador quer que ele tenha prioridade de
  novo depois, marca de novo.
- Se o operador precisa garantir uma **dupla específica** (não só a ordem),
  a via é montar o `Draft` manualmente (`create_team`) — aí os dois já
  estão reservados juntos, independente da ordem da lista.

<!--
Quando múltiplos critérios de pareamento entram em conflito, qual prevalece.
Ex: balanceamento de nível tem prioridade sobre variar parceiros.
-->

## Casos-limite conhecidos

- **Concorrência no pop da lista:** criar um `Draft` (automático ou manual)
  remove seus jogadores de `session_queue` na **mesma transação** — dois
  `report_match_result` concorrentes (quadras diferentes da mesma `Session`)
  não conseguem reservar o mesmo jogador pras duas quadras. É o único ponto
  do módulo com proteção transacional; o resto (`create_match` check + insert,
  etc.) ainda é TOCTOU teórico, aceito por não haver operadores simultâneos
  de fato.
- **`next_challenger` sem gente suficiente do gênero necessário** (`Mixed`
  desbalanceado, ou lista quase vazia): não forma `Draft` pra aquela quadra,
  ela fica ociosa e a resposta sinaliza. O operador monta manualmente
  (pode furar o gênero) ou espera a lista encher. Comportamento aceito.
- **Repetição de parceiro na sugestão automática:** `next_challenger` pega
  exatamente os N primeiros da lista e não tenta alternativas pra evitar
  repetir uma dupla que já jogou junta — só anota. Se os 2 primeiros da
  lista já foram parceiros, a sugestão os repete; o operador troca no
  `Draft` se quiser. Aceito (o balanceamento de parceria é responsabilidade
  do operador nesse modelo, não da lista).
- **Preenchimento guloso multi-quadra:** ver "Seeding inicial" em "Fila e
  rotação de quadra" — quadras ociosas são servidas por ordem de espera,
  não maximizando o nº de quadras ativas.

<!--
Situações especiais já discutidas/decididas. Ex: número ímpar de jogadores,
Session sem jogadores suficientes para as quadras disponíveis, jogador
removido de uma Session após os times já terem sido sorteados, etc.
-->

## Histórico de mudanças

Movido para [`CHANGELOG.md`](./CHANGELOG.md) — não é recarregado por padrão
junto com este arquivo; ler só quando for preciso o contexto histórico de
uma regra específica (motivo/data de uma decisão já refletida nas seções
acima).

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
  dupla com qualquer outro. `Open` só é válido combinado com
  `ShuffleType::RoundRobin` (ver seção seguinte); qualquer outra combinação
  é rejeitada por `GameMode::validate_shuffle_type`.
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
  usada conforme as partidas terminam: `KingAndQueen` (fila contínua com
  rotação por vitórias, descrita abaixo) ou `RoundRobin` (só válido com
  `GameMode::Open` — mesma mecânica de vitória/derrota do `KingAndQueen`,
  mas a fila prioriza fortemente formar duplas inéditas: um jogador liberado
  só completa um time incompleto da fila se essa dupla nunca jogou junta na
  `Session`; se todos os times incompletos esperando já jogaram com ele,
  abre um time novo em vez de repetir a dupla — nunca força a repetição
  enquanto existir alternativa. O sorteio *inicial* nesse modo continua
  puramente aleatório, sem gênero, igual aos outros modos — a garantia de
  "duplas inéditas" só entra em ação na fila, conforme as partidas
  terminam). O campo é explícito e obrigatório na `Session` (mesmo padrão do
  `GameMode`) para que nenhuma `Session` mude de comportamento
  silenciosamente.
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
  a sobra existente. Em `RoundRobin` (`GameMode::Open`), a compatibilidade
  também exige zero conflito de histórico com o jogador liberado
  (`GameMode::requires_fresh_partner`) — só entra num time incompleto que
  ainda não jogou com ele; caso contrário abre time novo, mesmo que isso
  signifique acumular mais de uma `Team` incompleta em paralelo. Implementado
  por `TeamQueueManager::release_players`
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
- **Toda quadra ociosa da `Session` é reconsiderada a cada resultado, não só
  a que acabou de liberar** — essencial em `Session`s com mais de uma
  quadra: a cada `POST /matchmaking/matches/{match_id}/result`,
  `resolve_match_result` varre todas as quadras que a `Session` já abriu
  alguma vez (via `Match.court` no histórico) e refaz a tentativa de
  preenchimento em qualquer uma cuja última partida já tenha resultado —
  não só a quadra do `Match` recém-reportado. Sem isso, uma quadra que
  ficou ociosa porque a fila estava rasa no momento em que *ela* tentou
  puxar um desafiante nunca seria revisitada depois, mesmo que a fila
  enchesse via resultados de *outras* quadras — ficaria travada
  indefinidamente. As quadras ociosas são preenchidas em ordem de "ociosa
  há mais tempo primeiro" (mesmo princípio FIFO da fila de jogadores),
  cada uma consumindo da mesma fila compartilhada sem repetir `Team`; uma
  quadra que a fila ainda não consegue preencher só é pulada (sem
  consumir nada), não trava as demais. **Trade-off aceito:** esse
  preenchimento guloso por ordem de espera não maximiza necessariamente o
  número de quadras ativas no fim da chamada — uma quadra antiga
  precisando de 2 `Team`s novas pode consumir as duas últimas `Team`s
  completas da fila e deixar uma quadra mais nova (precisando de só 1
  desafiante) ociosa, quando preenchê-la primeiro colocaria mais uma
  quadra em jogo agora. Prioriza justiça por tempo de espera sobre
  utilização total das quadras.
- Orquestrado por `TeamHandlerImpl::resolve_match_result`
  (`api/src/modules/matchmaking/handler/team.rs`), chamado por
  `MatchHandlerImpl::report_match_result`
  (`api/src/modules/matchmaking/handler/matches.rs`).
- **Entrada prioritária manual:** `POST /matchmaking/teams/priority`
  (`TeamHandlerImpl::create_priority_team`) monta uma `Team` a partir dos
  jogadores informados e a marca com `Team::with_priority`, o que a coloca
  na frente de toda `Team` não-prioritária em
  `TeamQueueManager::next_complete_teams` — é o próximo desafiante
  garantido assim que uma quadra ficar livre (não necessariamente *essa*
  quadra específica, se mais de uma abrir ao mesmo tempo). Diferente de
  `create_team`, essa via aceita puxar um jogador que já está em outra
  `Team` `Waiting` (contanto que ela não esteja disputando um `Match` em
  andamento): a `Team` de origem é desfeita e o(s) parceiro(s) que sobrou(aram)
  sem dupla volta(m) pra fila normalmente, via
  `TeamQueueManager::release_players` — o mesmo caminho usado para
  jogadores liberados por resultado de partida. Um jogador numa `Team`
  `Holding` (segurando quadra) ou já ocupada num `Match` não pode ser
  puxado. Validado por `TeamValidator::validate_priority_team`.

## Restrições

- Um jogador não pode aparecer em duas `Team`s da mesma `Session`.
- Um jogador não pode se repetir dentro da mesma `Team`.
- `GameMode::Mixed` exige `players_per_team` par (para dividir metade
  homens / metade mulheres por `Team`). Validado tanto na criação/edição da
  `Session` (`GameMode::validate_players_per_team`, chamado por
  `Session::new`/`set_settings`/`set_game_mode`) quanto no sorteio
  (`TeamDrawer::draw`), para que uma `Session` nunca fique salva numa
  configuração que o sorteio não consegue honrar.
- `GameMode::Open` só pode ser combinado com `ShuffleType::RoundRobin`, e
  vice-versa — qualquer outra combinação retorna `HttpError::bad_request`
  (`GameMode::validate_shuffle_type`). Validado em `Session::new`,
  `set_game_mode`, `set_shuffle_type` e `set_game_mode_and_shuffle_type`
  (esse último usado quando os dois campos mudam na mesma atualização, pra
  validar o par final em vez de passar por um estado intermediário
  inválido — ver `SessionHandlerImpl::update_session`).
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

- `POST /matchmaking/teams/` (`create_team`) é a via manual de entrada de
  `Team`: independente do `GameMode`/`ShuffleType` da `Session` (esses só
  restringem o sorteio automático e a rotação da fila, nunca a montagem
  manual), permite montar um time escolhendo jogadores específicos — usado
  como contingência quando o sorteio/fila automáticos precisam de correção
  manual. Ainda assim exige que todo `player_id` esteja confirmado em
  `Session::player_ids` e que nenhum já esteja em outra `Team` **ativa**
  (`Waiting`/`Holding`) da mesma `Session`; jogadores de uma `Team`
  `Disbanded` já estão livres de novo e não bloqueiam.

Restrições de duplicidade/elegibilidade de jogador validadas por
`TeamValidator::validate_new_team` (`api/src/modules/matchmaking/domain/team.rs`),
chamado tanto por `TeamHandlerImpl::create_team` quanto por
`TeamHandlerImpl::draw_teams`.

<!--
Condições que uma implementação NUNCA pode violar. Ex: número mínimo/máximo
de jogadores por Session, quadras disponíveis (available_courts) não podem
ser excedidas, etc.
-->

## Prioridades

- Uma `Team` marcada `priority` (via `POST /matchmaking/teams/priority`)
  sempre entra em quadra antes de qualquer `Team` não-prioritária, mesmo
  que essa última esteja esperando há mais tempo — a ordem por
  `created_at` (FIFO) só decide empate dentro do mesmo grupo (entre
  prioritárias, ou entre não-prioritárias). Ver
  `TeamQueueManager::next_complete_teams`.

<!--
Quando múltiplos critérios de pareamento entram em conflito, qual prevalece.
Ex: balanceamento de nível tem prioridade sobre variar parceiros.
-->

## Casos-limite conhecidos

- `draw_teams` faz o check de "sessão já tem times" e os inserts em
  chamadas separadas ao repositório — duas chamadas concorrentes para a
  mesma `Session` podem, em teoria, passar pelo check antes de qualquer
  insert acontecer (TOCTOU). Não tratado ainda (sem transação/lock em
  nenhum repositório do módulo). O repositório já é Postgres real (desde o
  PR #82 de `feat/matchmaking-sql-repository`), não mais em memória — a
  ressalva antiga de "aceitável por não ter carga concorrente real" descreve
  a ausência de operadores simultâneos de fato, não uma proteção técnica.
  Desde que `resolve_match_result` passou a reconsiderar todas as quadras
  ociosas da `Session` a cada resultado (não só a que acabou de liberar —
  ver "Continuação automática da quadra"), essa janela de corrida ficou bem
  mais larga: duas chamadas concorrentes de `report_match_result` para
  quadras *diferentes* da mesma `Session` agora podem disputar a mesma
  `Team` `Holding` ociosa antiga ou as mesmas `Team`s da fila, não só
  colidir consigo mesma na mesma quadra como antes.
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
- Em `RoundRobin`, pelo mesmo motivo (recusa ativamente completar um time
  que repetiria uma dupla), grupos pequenos com histórico de parceria muito
  concentrado podem acumular várias `Team`s incompletas simultâneas em vez
  de uma só. A decisão de quem completa qual time é local por jogador
  liberado (não um rebalanceamento global da fila), então não há garantia
  de que o resultado seja o arranjo com o mínimo global de duplas repetidas
  possível — só que nenhum jogador é forçado a repetir parceiro enquanto
  ele, individualmente, tiver uma alternativa disponível no momento em que
  é liberado. Comportamento aceito, não é bug.

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

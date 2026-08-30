# Histórico de mudanças — matchmaking

> Registro cronológico de decisões de regra de negócio, com data e motivo.
> Não é recarregado automaticamente com o `SKILL.md` — só ler quando for
> preciso entender o motivo/contexto histórico de uma regra específica.

- 2026-08-30 — **redesenho da fila: de fila de `Team`s pra lista de
  jogadores.** A fila deixou de ser uma coleção de `Team`s pré-formadas
  (`TeamQueueManager::release_players`, regras 1/2 de 2026-08-23) e passou a
  ser uma lista de jogadores individuais por `Session`
  (`matchmaking.session_queue`), ordenada por `pinned`, jogos disputados
  (`games_played` asc) e tempo de espera. Times só são formados no momento
  de entrar em quadra: ao reportar o resultado, `next_challenger` pega os
  primeiros da lista que fecham a composição de gênero, roda o
  `TeamDrawer::draw` só pra ordenar/anotar repetição, e cria um `Team`
  `Draft` — o operador revisa (pode editar o roster ou montar manual,
  furando o gênero se quiser), confirma e a partida começa. Motivo: dados
  reais de produção (`Session` ca58f864) mostraram que o modelo de fila de
  `Team`s não misturava no caso comum — com o pool do `release_players`
  restrito a quem está sem time completo, a dupla perdedora liberada era
  quase sempre a única opção do `TeamDrawer` e se reformava toda rodada. As
  tentativas de consertar isso dentro do modelo antigo (a "regra 3" de
  adiar repetição forçada, ver entrada abaixo) esbarravam em deadlock ou
  em não disparar no cenário real. O modelo de lista corta a classe inteira
  de bug: "quem entra" vira um `sort` + `take`, sem reagrupamento, sem
  fases, sem deadlock possível; a qualidade do pareamento passa a ser
  responsabilidade explícita do operador (que agora vê e confirma cada time
  antes de jogar). Remove `team_queue.rs` inteiro (`TeamQueueManager`,
  `release_players`, `select_playable`, `next_complete_teams`), o status
  `Team::Waiting`, `Team::with_priority`/`create_priority_team` (prioridade
  vira `pinned` na linha da lista) e `draw_teams` (vira `queue/seed` no
  mesmo fluxo formar→confirmar). `TeamDrawer` e `PartnerHistory` ficam, só
  como sugestão. Sem migração de dados — o app ainda não está em uso real,
  o DB foi resetado. Ver "Fila e rotação de quadra" no `SKILL.md`.
- 2026-08-30 — considerada e descartada uma 3ª regra pra
  `TeamQueueManager::release_players` que adiava uma repetição forçada de
  dupla (o grupo voltava a ser 2 `Team`s incompletas) quando já havia
  outra `Team` completa `Waiting` mantendo alguma quadra ocupada. Chegou a
  ser prototipada com guardas contra deadlock (só quando a liberação
  formava um único grupo, nunca um grupo com jogador `priority`, só quando
  existia a tal alternativa completa na fila), mas foi removida antes de
  mergear: reintroduzia justamente o tipo de bookkeeping de histórico de
  parceria + condição de escape que a reescrita de 2026-08-23 tirou, e
  carregava um trade-off de justiça/espera não resolvido — em `Session`
  multi-quadra com fila funda, a mesma dupla podia ser adiada rodada após
  rodada enquanto sempre sobrasse outra `Team` completa. Decisão: manter a
  fila governada só pelas 2 regras de 2026-08-23 e aceitar repetição
  forçada de dupla quando o `TeamDrawer` não acha alternativa no pool — é
  a mesma limitação estrutural que o sorteio inicial já tem quando sobra
  pouca gente pra formar o último grupo.
- 2026-08-25 — removido `ShuffleType` (`KingAndQueen`/`RoundRobin`) por
  completo: campo, validação cruzada com `GameMode`
  (`GameMode::validate_shuffle_type`), coluna `shuffle_type` na tabela
  `matchmaking.session` (migration
  `20260825133214_drop-session-shuffle-type.sql`), e o campo equivalente na
  API/schema/client do `my-app`. Desde a reescrita de 2026-08-23,
  `ShuffleType` já não tinha nenhum efeito de comportamento próprio —
  `Open ⇔ RoundRobin` era uma bijeção forçada por validação, então o valor
  era inteiramente derivável de `GameMode` (o próprio `SessionFormSheet.tsx`
  do frontend já derivava `shuffleType` a partir do `gameMode` escolhido, em
  vez de deixar o usuário escolher). Sem nenhuma lógica lendo o campo, ele
  só carregava dado redundante — removido como simplificação, não como
  mudança de regra de negócio.
- 2026-08-23 — reescrito `TeamQueueManager::release_players` do zero. O
  design anterior (jogadores liberados tentando completar `Team`s
  incompletas uma a uma, com `PartnerHistory` bloqueando repetição) passou
  por duas rodadas de patch no mesmo dia tentando corrigir "duplas
  repetindo demais" e depois "a fila trava de vez" (`already_waiting_ids`/
  `queue_has_slack`/fases 1 e 2, com `GameMode::Open` tratado à parte) — o
  último patch ainda travou de vez numa `Session` real de 9-10 jogadores em
  `Open`/`RoundRobin` (a exceção de `Open` na fase 2 significava que, uma
  vez esgotadas as combinações de dupla possíveis, a fila parava de formar
  qualquer `Team` completa, e sem `Team` completa nenhuma não há partida
  futura pra disparar uma nova chamada de `release_players` que desse uma
  segunda chance). Trocado por um algoritmo bem mais simples, sem fases
  nem `GameMode::Open` tratado à parte: a cada liberação, junta todo mundo
  sem `Team` completa (liberados agora + quem já esperava numa `Team`
  incompleta) num único grupo, seleciona dali o suficiente pra formar
  `Team`s completas — sempre os que esperam há mais tempo primeiro, pra
  ninguém "voltar imediatamente" — e entrega esse grupo pro mesmo
  `TeamDrawer::draw` do sorteio inicial, reaproveitando sua lógica de
  mistura já testada em vez de reimplementá-la. Remove
  `GameMode::requires_fresh_partner` (ficou sem uso) e deixa
  `ShuffleType::RoundRobin` sem nenhum efeito de comportamento próprio na
  fila (única diferença remanescente de `Open` é ignorar gênero, que já
  vinha do `GameMode`). Ver "Fila e rotação de quadra" no `SKILL.md` pro
  algoritmo atual. Revisão pelo `gb-matchmaking-domain-guardian` encontrou
  uma regressão real dessa reescrita antes de mergear: como a nova versão
  sempre desfaz e recria a `Team` (em vez de mutar in-place como antes), uma
  `Team` `priority` que estivesse incompleta perdia o flag `priority` ao
  ser completada — corrigido propagando `priority` de qualquer `Team`
  incompleta antiga que perca membro pra um grupo novo. Ver "Prioridades"
  no `SKILL.md`.
- 2026-08-18 — corrigido bug relatado em `Session`s com mais de uma quadra:
  `resolve_match_result` só recalculava a fila para a quadra do `Match`
  recém-reportado, então uma quadra que ficava ociosa por falta de `Team`s
  completas na fila nunca era revisitada por resultados reportados em
  *outras* quadras — ficava travada mesmo depois da fila encher. Passa a
  varrer, a cada resultado, todas as quadras que a `Session` já abriu (pela
  última partida de cada `court` no histórico), preenchendo as ociosas em
  ordem de tempo de espera (mais antiga primeiro) a partir da mesma fila
  compartilhada — inclui o caso em que nem o vencedor sobra (bateu o cap de
  vitórias com a fila rasa: nenhuma `Team` `Holding` ficaria pra "lembrar"
  que aquela quadra existe, então a detecção usa a última partida de cada
  quadra, não o status da `Team`). Ver "Continuação automática da quadra" e
  "Casos-limite conhecidos" (janela de TOCTOU mais larga, trade-off do
  preenchimento guloso).
- 2026-08-15 — nova rota `POST /matchmaking/teams/priority`
  (`TeamHandlerImpl::create_priority_team`) para o operador informar
  manualmente qual dupla joga a seguir, ignorando a fila FIFO normal —
  caso de uso: uma quadra está rodando e o operador quer garantir quem
  entra assim que ela liberar, sem depender do sorteio/fila automáticos.
  `Team` ganha o campo `priority` (`Team::with_priority`), que
  `TeamQueueManager::next_complete_teams` passa a priorizar sobre a ordem
  por `created_at`. Diferente de `create_team`, essa via pode puxar um
  jogador de outra `Team` `Waiting` não ocupada em partida — a `Team` de
  origem é desfeita e o parceiro que sobra é re-inserido na fila via
  `TeamQueueManager::release_players`, igual a um jogador liberado por
  resultado de partida.
- 2026-08-15 — `create_team` (entrada manual de `Team`) passa a validar que
  todo `player_id` está confirmado em `Session::player_ids`
  (`HttpError::bad_request` caso contrário), e o check de "já está em outro
  time" passa a ignorar `Team`s `Disbanded` (antes bloqueava indevidamente
  a reentrada manual de um jogador cujo time anterior já havia se desfeito
  — exatamente o cenário de contingência que essa rota existe pra cobrir).
  Confirma que a rota continua deliberadamente sem checagem de `GameMode`/
  `ShuffleType`: a montagem manual é independente da configuração da
  `Session`.
- 2026-08-14 — adicionado `GameMode::Open` (ignora gênero na formação de
  times) e `ShuffleType::RoundRobin` (mesma mecânica de fila/quadra do
  `KingAndQueen`, mas a fila prioriza fortemente duplas inéditas ao
  completar times incompletos, abrindo time novo em vez de repetir parceiro
  quando há alternativa). Os dois só são válidos combinados entre si
  (`GameMode::validate_shuffle_type`). O sorteio inicial nesse modo continua
  aleatório sem garantia especial (histórico sempre vazio nesse ponto); a
  garantia de "duplas inéditas" vale só pra fila, conforme partidas
  terminam (`TeamQueueManager::release_players`).
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

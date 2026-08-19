# Histórico de mudanças — matchmaking

> Registro cronológico de decisões de regra de negócio, com data e motivo.
> Não é recarregado automaticamente com o `SKILL.md` — só ler quando for
> preciso entender o motivo/contexto histórico de uma regra específica.

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

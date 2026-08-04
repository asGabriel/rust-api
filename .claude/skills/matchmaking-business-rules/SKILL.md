---
name: matchmaking-business-rules
description: Regras de negócio do módulo de matchmaking — critérios de pareamento, restrições, prioridades e casos-limite.
---

# Regras de negócio — matchmaking

> Módulo ainda no início da implementação. Este documento é um esqueleto: as
> seções abaixo devem ser preenchidas manualmente conforme as regras forem
> sendo decididas e implementadas em `api/src/modules/matchmaking`. Não
> inferir regras a partir do código atual — só documentar aqui o que for
> explicitamente decidido como regra de negócio.

## Critérios de pareamento

_A definir._

<!--
Como jogadores são agrupados em duplas/times (Team) e como partidas
(Match) são formadas a partir das duplas de uma Session. Ex: nível de
habilidade, histórico de parceria, aleatoriedade controlada, etc.
-->

## Restrições

- Um jogador não pode aparecer em duas `Team`s da mesma `Session`.
- Um jogador não pode se repetir dentro da mesma `Team`.

Ambas validadas por `TeamValidator::validate_new_team`
(`api/src/modules/matchmaking/domain/team.rs`), chamado por
`TeamHandlerImpl::create_team`.

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

_A definir._

<!--
Situações especiais já discutidas/decididas. Ex: número ímpar de jogadores,
Session sem jogadores suficientes para as quadras disponíveis, jogador
removido de uma Session após os times já terem sido sorteados, etc.
-->

## Histórico de mudanças

- 2026-08-04 — implementada validação em `create_team`: rejeita jogador
  duplicado dentro da mesma `Team` (`HttpError::bad_request`) e jogador já
  presente em outra `Team` da mesma `Session` (`HttpError::conflict`).

<!--
Registro cronológico de decisões de regra de negócio, com data e motivo.
Ex: "2026-08-04 — decidido que sorteio de duplas é aleatório sem
balanceamento de nível na v1, ver discussão em <link/PR>."
-->

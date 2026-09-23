# Histórico de mudanças — finance_manager

> Registro cronológico de decisões de regra de negócio, com data e motivo.
> Não é recarregado automaticamente com o `SKILL.md` — só ler quando for
> preciso entender o motivo/contexto histórico de uma regra específica.

- 2026-09-08 — criado o skill `gb-finance-business-rules` e o agent
  `gb-finance-domain-guardian`, espelhando o par já existente do
  matchmaking. O skill nasce com todas as seções "a definir": a
  implementação atual de `finance_manager` deixou de atender às
  necessidades do dono, então as regras serão redefinidas do zero por
  entrevista, e não documentadas a partir do código existente.
- 2026-09-08 — **Dívida: removido `discount_amount`** do modelo (não há
  mais desconto); invariante vira `remaining_amount = total_amount -
  paid_amount`, nenhum valor negativo, `paid_amount` nunca passa de
  `total_amount`.
- 2026-09-08 — **Parcelamento remodelado na tabela `debt` via `parent_id`.**
  Sai a tabela `debt_installment`; um plano vira uma dívida-pai +
  N dívidas-filhas (`parent_id`), cada filha uma dívida completa. Só as
  filhas são pagáveis (pai deriva paid/remaining/status da soma das
  filhas); filhas são congeladas na criação (sem edição individual);
  resto do arredondamento na última filha; parcelar só na criação. Motivo:
  reaproveitar toda a máquina de pagamento/status/soft-delete/filtros da
  dívida em vez de manter um agregado paralelo, e eliminar o
  special-casing installment↔payment (`payment_id` na parcela, os dois
  branches de `process_debt_payment`, boa parte do `pubsub.rs`). Removido
  o status `DebtStatus::Installment` — "ser parcelamento" é ter filhas.
  Removido o endpoint `/debt/installment/list` (vira `list_debts` com
  filtro `parent_id`).
- 2026-09-15 — **`DebtCategory`: removido `Personal`, adicionados
  `Subscriptions`, `Obligations` e `Purchases`.** Motivo: `Personal` era
  genérico demais; vira três categorias mais específicas — assinaturas
  recorrentes, impostos/dívidas formais e compras em geral (esta última
  cobre o que antes caía em `Personal`).
- 2026-09-21 — **Filtro `leavesOnly` e `installment_count` copiado nas
  filhas.** Motivo: a tela da competência (mês) precisa listar o que é
  pagável no período; o pai tem `due_date = NULL` e não aparece num filtro
  por data, e mostrar só o pai ficaria estranho. `leavesOnly` devolve
  comuns + parcelas em lista plana; copiar `installment_count` nas filhas
  permite exibir "k/N" sem buscar o pai.
- 2026-09-21 — **`leavesOnly` revertido e trocado por `includeChildren`.**
  `leavesOnly` era direcionado demais à tela da competência. O flag
  `includeChildren` é mais neutro: só remove a restrição de nível-topo,
  devolvendo pais e parcelas numa lista plana. Mantida a cópia de
  `installment_count` nas filhas.
- 2026-09-23 — **Removido instrumento financeiro do escopo.** O módulo
  novo `finance` não tem (nem reserva campo para) instrumento financeiro
  por ora — `Payment` e `Debt` não referenciam conta/instrumento. A seção
  "Instrumento financeiro" sai do skill e das áreas validadas pelo
  guardian; volta só se o conceito for reintroduzido por decisão explícita.

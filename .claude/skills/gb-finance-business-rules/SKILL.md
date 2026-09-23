---
name: gb-finance-business-rules
description: Regras de negócio do módulo finance_manager — invariantes de domínio e validação de dívida, pagamento, parcela, recorrência, fatura, receita e instrumento financeiro.
---

# Regras de negócio — finance_manager

> Documento mantido **manualmente** pelo dono do projeto. As seções abaixo
> começam vazias de propósito: a implementação atual em
> `api/src/modules/finance_manager` **não** reflete mais as regras
> desejadas, então nada aqui deve ser inferido do código. Cada regra é
> preenchida por decisão explícita (via entrevista com o dono) e só então
> passa a valer como fonte da verdade para o `gb-finance-domain-guardian`.
>
> Enquanto uma seção estiver "a definir", trate-a como **sem regra formal**:
> o guardian não valida contra ela e sinaliza que a área ainda não foi
> documentada, em vez de inventar regra.

Escopo: invariantes de domínio e validação. **Fora de escopo** (não
documentar aqui): convenções estruturais do workspace (domain/handler/
repository/routes), auth/scoping transversal, paginação, panics — isso é
responsabilidade de review geral, não deste skill.

---

### Invariantes de valores

- Campos monetários: `total_amount`, `paid_amount`, `remaining_amount`.
  **`discount_amount` foi removido** — não existe mais desconto no modelo.
- Invariante central: `remaining_amount = total_amount - paid_amount`.
- `0 <= paid_amount <= total_amount` — um pagamento nunca leva
  `paid_amount` acima de `total_amount`.
- `0 <= remaining_amount <= total_amount`.
- Nenhum dos três valores pode ser negativo.
- `total_amount > 0` na criação.
- `remaining_amount` é sempre **recalculado** a partir de `total_amount` e
  `paid_amount` (a cada pagamento/estorno); nunca é setado diretamente pela
  API nem persistido de forma independente.
- _A definir:_ editar `total_amount` de uma dívida comum depois que já
  existe pagamento é permitido? (provável: proibido.)

### Categoria (`DebtCategory`)

- Variantes: `Unknown` (default), `Home`, `Transport`, `Health`, `Food`,
  `Lifestyle`, `Education`, `Goals`, `Subscriptions`, `Obligations`,
  `Purchases`.
- **`Personal` foi removido.** No lugar entram três categorias mais
  específicas: `Subscriptions` (assinaturas recorrentes — streaming,
  software, etc.), `Obligations` (impostos e dívidas/empréstimos formais —
  IPTU, IPVA, IR, financiamento, cartório) e `Purchases` (compras em geral —
  o catálogo genérico que antes caía em `Personal`).
- Na filha de um parcelamento, `category` é copiada do pai na criação (ver
  Parcelamento) — não é escolhida independentemente.

### Status e transições

- Estados: `Open`, `Settled`. **`Installment` foi removido como status** —
  "ser um parcelamento" é ter filhos (`parent_id`), não um estado.
- `Open -> Settled` quando `paid_amount == total_amount`.
- `Settled -> Open` quando um estorno reduz `paid_amount` abaixo de
  `total_amount`.
- **Dívida-pai:** status é **derivado dos filhos** — `Settled` se todas as
  filhas estão `Settled`, senão `Open`. O pai nunca recebe pagamento
  direto.
- _A definir:_ existe um `Overdue` derivado de `due_date` vs. hoje, ou isso
  é só concern de view (não persistido)?

### Parcelamento (`parent_id`)

Parcelamento é modelado **na própria tabela `debt`**, não em tabela
separada. `debt.debt_installment` deixa de existir.

- `debt.parent_id UUID NULL` — auto-referência para `debt(id)`. Profundidade
  máxima 1: uma dívida-filha nunca tem filhas.
- **Dívida comum:** `parent_id = NULL`, sem filhas.
- **Parcelamento:** uma **dívida-pai** (`parent_id = NULL`, `total_amount` =
  valor cheio, `installment_count = N`) + **N dívidas-filhas**
  (`parent_id = <pai>`), cada uma uma dívida completa com `due_date`,
  `total_amount`, `paid_amount`, `remaining_amount` e `status` próprios.
- `installment_count` fica **na dívida-pai** (mantido explicitamente, não
  derivado de `count(filhos)`). As filhas também o **recebem copiado** na
  geração, só para exibição ("k/N"); é imutável e não é fonte da verdade.
  Consequência: "é pai" = `parent_id IS NULL AND installment_count IS NOT
  NULL`.
- Invariante: `soma(filhas.total_amount) == pai.total_amount`, garantido na
  geração. `pai.total_amount` é **imutável** enquanto houver filhas.
- **Divisão do valor:** `total_amount / N` como valor base de cada filha,
  quantizado em 2 casas decimais (bater com `DECIMAL(10,2)`); o resto em
  centavos vai na **última** filha.
- Cada filha tem `due_date` própria: mensal, a partir da data de vencimento
  informada na criação; quando o mês não tem o dia, usa o último dia válido
  do mês.
- `pai.due_date = NULL` — não significa nada no fluxo de parcelamento.
- **Pagamento:** só as filhas são pagáveis. `POST /payment` contra uma
  dívida que tem filhas é rejeitado. `pai.paid_amount` / `pai.remaining_amount`
  são derivados da soma das filhas na leitura (não persistidos de forma
  independente — _a definir_ se persiste em sync).
- **Sem ordem obrigatória:** qualquer filha pode ser paga a qualquer
  momento; não há regra de "parcela k só depois da k-1".
- **Filhas congeladas:** não há rota de edição de filha. `due_date` e
  `total_amount` de uma filha são definidos uma vez na geração. Só
  `paid_amount` / `remaining_amount` / `status` se movem, e só via
  pagamento/estorno. Restruturar um plano = cancelar o pai (cascade) e
  criar outro.
- Filhas **copiam** do pai na criação: `client_id`, `category`, `list_id`,
  `expense_type`, `description`. Edições posteriores no pai **não**
  propagam para as filhas — **exceto `list_id`**: a lista é só um
  agrupador (não é dado financeiro), e são as filhas que aparecem mês a
  mês, então vincular/desvincular o pai a uma lista propaga para todas as
  filhas não-deletadas, na mesma transação.
- **Lista (`list_id`):** no `PATCH` da dívida, campo ausente = mantém,
  `null` = desvincula, uuid = vincula. Filha continua não-editável
  diretamente (o vínculo de uma parcela muda pelo pai).
- Ordinal da parcela: filha guarda `installment_number` (1..N) para
  ordenação/exibição ("3/12"); `NULL` em dívida não-parcelada.
- Parcelar só acontece **na criação** da dívida. Não há converter uma
  dívida comum existente em parcelamento.
- **Listagem:** `DebtFilters` passa a filtrar por `parent_id` — o "listar
  dívidas" default traz só nível-topo (`parent_id IS NULL`);
  `parent_id = X` lista as parcelas de X. O endpoint
  `/debt/installment/list` deixa de existir.
- **Flag `includeChildren`:** `DebtFilters.include_children = true` remove a
  restrição de nível-topo e devolve, numa lista plana, dívidas de
  nível-topo **e** parcelas. Usado na tela da competência (mês): o filtro de
  data pega as parcelas do período (o pai tem `due_date = NULL` e não casa
  com filtro de data). Sem filtro de data a lista mistura pais e parcelas —
  o consumidor distingue por `parent_id`. Com `false`/ausente vale o
  comportamento default acima.
- **Soft-delete:** apagar a dívida-pai faz cascade nas filhas e nos
  pagamentos das filhas.

---

## Pagamento (`Payment`)

### Validação do valor

_A definir._ (valor mínimo/máximo aceito, relação com o remaining da dívida
ou com o valor da parcela, pagamento parcial permitido ou não.)

### Quitação

_A definir._ (quando a dívida é considerada quitada, pagamento que excede o
saldo.)

### Conciliação (`reconcile`)

_A definir._ (o que "reconcile" deve significar como regra de negócio, o que
pode ser sobrescrito na dívida, se validação se aplica nesse caminho.)

### Estorno (`refund`)

_A definir._ (o que pode ser estornado, efeito na dívida e nas parcelas,
soft-delete vs. hard-delete como decisão de negócio, estorno parcial.)

---

## Parcela (`Installment`)

_A definir._ (ordem de pagamento entre parcelas, valor exato vs. aproximado,
o que acontece ao estornar o pagamento de uma parcela, parcela vencida.)

---

## Recorrência (`Recurrence`)

_A definir._ (janela de validade start/end, idempotência por mês, o que a
dívida gerada herda do template, o que acontece quando a recorrência é
desativada ou editada no meio, escopo por cliente da geração.)

---

## Fatura (`Invoice`)

_A definir._ (mês de competência, elegibilidade de uma dívida para ser
vinculada, uma dívida pode estar em mais de uma fatura, o que acontece com o
vínculo quando a dívida é removida, propriedade/ownership.)

---

## Receita (`Income`)

_A definir._

---

## Instrumento financeiro (`FinancialInstrument`)

_A definir._ (tipos e o que cada um exige, configuração obrigatória por
tipo, o que significa `default_due_date`, edição.)

---

## Escopo e propriedade (`client_id`)

_A definir._ (o que cada operação precisa validar sobre a posse do recurso
pelo cliente autenticado; que erro retornar quando falha — not_found vs.
forbidden.)

---

## Casos-limite conhecidos

_A definir._ (situações especiais já discutidas e decididas.)

---

## Histórico de mudanças

Movido para [`CHANGELOG.md`](./CHANGELOG.md) — não é recarregado por padrão
junto com este arquivo; ler só quando for preciso o contexto histórico de
uma regra específica (motivo/data de uma decisão já refletida nas seções
acima).

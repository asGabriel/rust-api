ALTER TABLE finance_manager.debt
DROP CONSTRAINT IF EXISTS chk_debt_installment_requires_instrument;

DROP INDEX IF EXISTS finance_manager.idx_debt_financial_instrument_id;

ALTER TABLE finance_manager.debt
DROP COLUMN IF EXISTS financial_instrument_id;

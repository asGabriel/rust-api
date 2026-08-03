-- Add optional financial_instrument_id to debt; required when installment_count is set
ALTER TABLE finance_manager.debt
ADD COLUMN financial_instrument_id UUID NULL REFERENCES finance_manager.financial_instrument(id);

-- Backfill existing installment debts so the CHECK constraint can be applied
UPDATE finance_manager.debt
SET financial_instrument_id = '185005a4-831e-4b46-a9cc-d19ad9dfb574'
WHERE installment_count IS NOT NULL AND installment_count > 0;

ALTER TABLE finance_manager.debt
ADD CONSTRAINT chk_debt_installment_requires_instrument
CHECK (
    (installment_count IS NULL OR installment_count <= 0)
    OR (financial_instrument_id IS NOT NULL)
);

CREATE INDEX idx_debt_financial_instrument_id ON finance_manager.debt(financial_instrument_id);

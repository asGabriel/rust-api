-- Add optional financial_instrument_id to debt; required when installment_count is set
ALTER TABLE finance_manager.debt
ADD COLUMN financial_instrument_id UUID NULL REFERENCES finance_manager.financial_instrument(id);

ALTER TABLE finance_manager.debt
ADD CONSTRAINT chk_debt_installment_requires_instrument
CHECK (
    (installment_count IS NULL OR installment_count <= 0)
    OR (financial_instrument_id IS NOT NULL)
);

CREATE INDEX idx_debt_financial_instrument_id ON finance_manager.debt(financial_instrument_id);

-- Soft delete metadata
ALTER TABLE finance_manager.debt
    ADD COLUMN deleted_by JSONB NULL;

ALTER TABLE finance_manager.payment
    ADD COLUMN deleted_by JSONB NULL;

ALTER TABLE finance_manager.debt_installment
    ADD COLUMN deleted_by JSONB NULL;

CREATE INDEX idx_debt_client_id_active ON finance_manager.debt (client_id)
    WHERE deleted_by IS NULL;

CREATE INDEX idx_payment_client_id_active ON finance_manager.payment (client_id)
    WHERE deleted_by IS NULL;

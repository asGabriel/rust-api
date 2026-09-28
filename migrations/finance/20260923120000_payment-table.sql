CREATE TABLE finance.payment (
    id UUID PRIMARY KEY,
    client_id UUID NOT NULL,
    debt_id UUID NOT NULL REFERENCES finance.debt(id),
    amount DECIMAL(10, 2) NOT NULL CHECK (amount > 0),
    payment_date DATE NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NULL,
    deleted_by JSONB NULL
);

CREATE INDEX idx_finance_payment_debt_id_active ON finance.payment(debt_id) WHERE deleted_by IS NULL;
CREATE INDEX idx_finance_payment_client_id_active ON finance.payment(client_id) WHERE deleted_by IS NULL;
CREATE INDEX idx_finance_payment_payment_date ON finance.payment(payment_date);

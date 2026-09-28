CREATE TABLE finance.income (
    id UUID PRIMARY KEY,
    client_id UUID NOT NULL,
    category TEXT NOT NULL,
    description TEXT NOT NULL,
    amount DECIMAL(10, 2) NOT NULL CHECK (amount > 0),
    received_date DATE NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NULL,
    deleted_by JSONB NULL
);

CREATE INDEX idx_finance_income_client_id_active ON finance.income(client_id) WHERE deleted_by IS NULL;
CREATE INDEX idx_finance_income_received_date ON finance.income(received_date);

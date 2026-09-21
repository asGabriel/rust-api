CREATE SCHEMA IF NOT EXISTS finance;

CREATE TABLE finance.debt (
    id UUID PRIMARY KEY,
    client_id UUID NOT NULL,
    identification SERIAL NOT NULL,
    category TEXT NOT NULL,
    expense_type TEXT NOT NULL,
    tags TEXT[] NOT NULL DEFAULT '{}',
    description TEXT NOT NULL,
    total_amount DECIMAL(10, 2) NOT NULL CHECK (total_amount > 0),
    paid_amount DECIMAL(10, 2) NOT NULL DEFAULT 0
        CHECK (paid_amount >= 0 AND paid_amount <= total_amount),
    remaining_amount DECIMAL(10, 2) NOT NULL
        CHECK (remaining_amount >= 0 AND remaining_amount <= total_amount),
    due_date DATE NULL,
    status TEXT NOT NULL,
    installment_count INT NULL,
    parent_id UUID NULL REFERENCES finance.debt(id),
    installment_number INT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NULL,
    deleted_by JSONB NULL
);

CREATE UNIQUE INDEX idx_finance_debt_identification ON finance.debt(identification);
CREATE INDEX idx_finance_debt_client_id_active ON finance.debt(client_id) WHERE deleted_by IS NULL;
CREATE INDEX idx_finance_debt_parent_id ON finance.debt(parent_id);
CREATE INDEX idx_finance_debt_due_date ON finance.debt(due_date);
CREATE INDEX idx_finance_debt_status ON finance.debt(status);
CREATE INDEX idx_finance_debt_client_status ON finance.debt(client_id, status);

CREATE TABLE finance.list (
    id UUID PRIMARY KEY,
    client_id UUID NOT NULL,
    name TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NULL
);

CREATE INDEX idx_finance_list_client_id ON finance.list(client_id);

ALTER TABLE finance.debt
    ADD COLUMN list_id UUID NULL REFERENCES finance.list(id) ON DELETE SET NULL;

CREATE INDEX idx_finance_debt_list_id ON finance.debt(list_id);

ALTER TABLE finance.debt DROP COLUMN tags;

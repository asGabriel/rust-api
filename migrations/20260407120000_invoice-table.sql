CREATE TABLE IF NOT EXISTS finance_manager.invoice (
    id UUID PRIMARY KEY,
    client_id UUID NOT NULL REFERENCES finance_manager.client_information(client_id),
    name TEXT NOT NULL,
    related_debt_ids UUID[] NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NULL,
    deleted_by JSONB NULL
);

CREATE INDEX IF NOT EXISTS idx_invoice_client_id ON finance_manager.invoice(client_id);

CREATE TABLE IF NOT EXISTS finance_manager.client_information (
    client_id UUID PRIMARY KEY,
    description TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NULL
);

CREATE INDEX IF NOT EXISTS idx_client_information_client_id ON finance_manager.client_information(client_id);
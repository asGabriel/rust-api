CREATE TABLE finance_manager.income (
    id UUID PRIMARY KEY,
    account_id UUID NOT NULL REFERENCES finance_manager.account(id),
    description TEXT NOT NULL,
    amount DECIMAL(10, 2) NOT NULL,
    reference DATE NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NULL
);

CREATE INDEX idx_income_account_id ON finance_manager.income(account_id);
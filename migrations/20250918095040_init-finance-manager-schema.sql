CREATE SCHEMA IF NOT EXISTS finance_manager;

CREATE TABLE finance_manager.account (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    owner TEXT NOT NULL,
    identification SERIAL NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NULL
);

CREATE TABLE finance_manager.debt (
    id UUID PRIMARY KEY,
    account_id UUID NOT NULL REFERENCES finance_manager.account(id),
    identification SERIAL NOT NULL,
    description TEXT NOT NULL,
    total_amount DECIMAL(10, 2) NOT NULL,
    paid_amount DECIMAL(10, 2) NOT NULL,
    discount_amount DECIMAL(10, 2) NOT NULL,
    remaining_amount DECIMAL(10, 2) NOT NULL,
    due_date DATE NOT NULL,
    status TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NULL
);

CREATE TABLE finance_manager.payment (
    id UUID PRIMARY KEY,
    debt_id UUID NOT NULL REFERENCES finance_manager.debt(id),
    account_id UUID NOT NULL REFERENCES finance_manager.account(id),
    total_amount DECIMAL(10, 2) NOT NULL,
    principal_amount DECIMAL(10, 2) NOT NULL,
    discount_amount DECIMAL(10, 2) NOT NULL,
    payment_date DATE NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NULL
);

-- Indexes for the account table
CREATE UNIQUE INDEX idx_account_identification ON finance_manager.account(identification);

-- Indexes for the debt table
CREATE UNIQUE INDEX idx_debt_identification ON finance_manager.debt(identification);
CREATE INDEX idx_debt_account_id ON finance_manager.debt(account_id);
CREATE INDEX idx_debt_due_date ON finance_manager.debt(due_date);
CREATE INDEX idx_debt_status ON finance_manager.debt(status);

-- Indexes for the payment table
CREATE INDEX idx_payment_debt_id ON finance_manager.payment(debt_id);
CREATE INDEX idx_payment_account_id ON finance_manager.payment(account_id);

-- Useful composite indexes
CREATE INDEX idx_debt_account_status ON finance_manager.debt(account_id, status);
CREATE INDEX idx_debt_status_due_date ON finance_manager.debt(status, due_date);
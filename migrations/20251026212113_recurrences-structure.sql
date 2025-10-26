CREATE TABLE finance_manager.recurrence (
    id UUID PRIMARY KEY,
    account_id UUID NOT NULL REFERENCES finance_manager.account(id),
    description TEXT NOT NULL,
    amount DECIMAL(10, 2) NOT NULL,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    start_date DATE NOT NULL,
    end_date DATE NULL,
    day_of_month INT NOT NULL,
    next_run_date DATE NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NULL
);

CREATE TABLE finance_manager.recurrence_run (
    id UUID PRIMARY KEY,
    recurrence_id UUID NOT NULL REFERENCES finance_manager.recurrence(id),
    debt_id UUID NOT NULL REFERENCES finance_manager.debt(id),
    run_date DATE NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NULL
);
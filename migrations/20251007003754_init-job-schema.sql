CREATE SCHEMA IF NOT EXISTS job;

CREATE TABLE job.payment_generator (
    id UUID PRIMARY KEY,
    payment_id UUID REFERENCES finance_manager.payment(id),
    debt_id UUID NOT NULL REFERENCES finance_manager.debt(id),
    status TEXT NOT NULL DEFAULT 'PENDING',
    history_logs JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NULL
);

ALTER TABLE job.payment_generator ADD CONSTRAINT uk_payment_generator_payment_debt UNIQUE (payment_id, debt_id);
ALTER TABLE job.payment_generator ADD CONSTRAINT fk_payment_generator_debt_id FOREIGN KEY (debt_id) REFERENCES finance_manager.debt(id);
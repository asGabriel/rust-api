CREATE TABLE IF NOT EXISTS finance_manager.debt_installment (
    debt_id UUID NOT NULL REFERENCES finance_manager.debt(id) ON DELETE CASCADE,
    installment_id INT NOT NULL,
    due_date DATE NOT NULL,
    amount DECIMAL(10, 2) NOT NULL,
    is_paid BOOLEAN NOT NULL DEFAULT FALSE,
    payment_id UUID NULL REFERENCES finance_manager.payment(id),
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NULL,
    
    PRIMARY KEY (debt_id, installment_id)
);

ALTER TABLE finance_manager.debt ADD COLUMN installment_count INT NULL;
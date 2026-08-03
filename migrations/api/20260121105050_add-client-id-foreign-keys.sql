-- Add foreign key constraints to ensure client_id references client_information table

ALTER TABLE auth.users
ADD CONSTRAINT fk_users_client_id
FOREIGN KEY (client_id) REFERENCES finance_manager.client_information(client_id);

ALTER TABLE finance_manager.financial_instrument
ADD CONSTRAINT fk_financial_instrument_client_id
FOREIGN KEY (client_id) REFERENCES finance_manager.client_information(client_id);

ALTER TABLE finance_manager.debt
ADD CONSTRAINT fk_debt_client_id
FOREIGN KEY (client_id) REFERENCES finance_manager.client_information(client_id);

ALTER TABLE finance_manager.income
ADD CONSTRAINT fk_income_client_id
FOREIGN KEY (client_id) REFERENCES finance_manager.client_information(client_id);

ALTER TABLE finance_manager.payment
ADD CONSTRAINT fk_payment_client_id
FOREIGN KEY (client_id) REFERENCES finance_manager.client_information(client_id);

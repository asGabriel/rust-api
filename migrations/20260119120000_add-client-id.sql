-- Add client_id to users table
ALTER TABLE auth.users ADD COLUMN client_id UUID NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000';
CREATE INDEX idx_users_client_id ON auth.users(client_id);

-- Add client_id to account table
ALTER TABLE finance_manager.account ADD COLUMN client_id UUID NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000';
CREATE INDEX idx_account_client_id ON finance_manager.account(client_id);

-- Add client_id to debt table
ALTER TABLE finance_manager.debt ADD COLUMN client_id UUID NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000';
CREATE INDEX idx_debt_client_id ON finance_manager.debt(client_id);

-- Add client_id to income table
ALTER TABLE finance_manager.income ADD COLUMN client_id UUID NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000';
CREATE INDEX idx_income_client_id ON finance_manager.income(client_id);

-- Add client_id to payment table
ALTER TABLE finance_manager.payment ADD COLUMN client_id UUID NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000';
CREATE INDEX idx_payment_client_id ON finance_manager.payment(client_id);

-- Remove default after migration (you'll set client_id manually)
ALTER TABLE auth.users ALTER COLUMN client_id DROP DEFAULT;
ALTER TABLE finance_manager.account ALTER COLUMN client_id DROP DEFAULT;
ALTER TABLE finance_manager.debt ALTER COLUMN client_id DROP DEFAULT;
ALTER TABLE finance_manager.income ALTER COLUMN client_id DROP DEFAULT;
ALTER TABLE finance_manager.payment ALTER COLUMN client_id DROP DEFAULT;

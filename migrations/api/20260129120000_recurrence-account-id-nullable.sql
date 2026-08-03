-- Make account_id nullable in recurrence table
ALTER TABLE finance_manager.recurrence 
ALTER COLUMN account_id DROP NOT NULL;

-- Update foreign key to reference financial_instrument (renamed from account)
ALTER TABLE finance_manager.recurrence 
DROP CONSTRAINT IF EXISTS recurrence_account_id_fkey;

ALTER TABLE finance_manager.recurrence 
ADD CONSTRAINT recurrence_account_id_fkey 
FOREIGN KEY (account_id) REFERENCES finance_manager.financial_instrument(id);

-- Add execution_logs as JSONB column
ALTER TABLE finance_manager.recurrence 
ADD COLUMN execution_logs JSONB NOT NULL DEFAULT '[]'::jsonb;

-- Add client_id to recurrence table
ALTER TABLE finance_manager.recurrence 
ADD COLUMN client_id UUID NOT NULL;

CREATE INDEX idx_recurrence_client_id ON finance_manager.recurrence(client_id);

ALTER TABLE finance_manager.recurrence 
ALTER COLUMN client_id DROP DEFAULT;

ALTER TABLE finance_manager.recurrence
ADD CONSTRAINT fk_recurrence_client_id
FOREIGN KEY (client_id) REFERENCES finance_manager.client_information(client_id);

-- Drop recurrence_run table (data now stored as JSONB in recurrence.execution_logs)
DROP TABLE IF EXISTS finance_manager.recurrence_run;

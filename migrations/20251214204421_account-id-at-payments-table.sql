-- Drop indexes that depend on account_id column
DROP INDEX IF EXISTS finance_manager.idx_debt_account_id;
DROP INDEX IF EXISTS finance_manager.idx_debt_account_status;

-- Drop the account_id column (this will also drop the foreign key constraint)
ALTER TABLE finance_manager.debt DROP COLUMN IF EXISTS account_id;


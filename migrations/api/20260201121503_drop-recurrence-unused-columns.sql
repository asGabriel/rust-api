-- Drop unused columns from recurrence table
ALTER TABLE finance_manager.recurrence DROP COLUMN IF EXISTS account_id;
ALTER TABLE finance_manager.recurrence DROP COLUMN IF EXISTS next_run_date;

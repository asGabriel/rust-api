-- Rename table from account to financial_instrument
ALTER TABLE finance_manager.account RENAME TO financial_instrument;

-- Add instrument_type column as TEXT with default value
ALTER TABLE finance_manager.financial_instrument 
ADD COLUMN instrument_type TEXT NOT NULL DEFAULT 'DEBIT_ACCOUNT';

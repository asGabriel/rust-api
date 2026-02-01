-- Rename account_id to financial_instrument_id in income table
ALTER TABLE finance_manager.income 
RENAME COLUMN account_id TO financial_instrument_id;

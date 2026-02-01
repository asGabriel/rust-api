-- Add category column to recurrence table
ALTER TABLE finance_manager.recurrence 
ADD COLUMN IF NOT EXISTS category TEXT NOT NULL DEFAULT 'UNKNOWN';

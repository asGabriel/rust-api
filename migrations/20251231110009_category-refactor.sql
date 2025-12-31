ALTER TABLE finance_manager.debt DROP CONSTRAINT IF EXISTS fk_debt_category_name;

ALTER TABLE finance_manager.debt RENAME COLUMN category_name TO category;
ALTER TABLE finance_manager.debt ALTER COLUMN category TYPE TEXT;

DROP INDEX IF EXISTS finance_manager.idx_debt_category_name;

DROP TABLE IF EXISTS finance_manager.debt_category;

ALTER TABLE finance_manager.debt ADD COLUMN tags TEXT[] DEFAULT '{}';
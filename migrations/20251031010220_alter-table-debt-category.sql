DROP INDEX IF EXISTS finance_manager.idx_debt_category_name;

UPDATE finance_manager.debt_category SET name = UPPER(name);

CREATE UNIQUE INDEX idx_debt_category_name_unique ON finance_manager.debt_category(name);

ALTER TABLE finance_manager.debt DROP COLUMN IF EXISTS category_id;
ALTER TABLE finance_manager.debt ADD COLUMN category_name TEXT;

UPDATE finance_manager.debt SET category_name = 'CASA' WHERE category_name IS NULL;
UPDATE finance_manager.debt SET category_name = UPPER(category_name) WHERE category_name IS NOT NULL;

ALTER TABLE finance_manager.debt_category DROP COLUMN IF EXISTS identification;

ALTER TABLE finance_manager.debt 
    ADD CONSTRAINT fk_debt_category_name 
    FOREIGN KEY (category_name) 
    REFERENCES finance_manager.debt_category(name);

CREATE INDEX IF NOT EXISTS idx_debt_category_name ON finance_manager.debt(category_name);

ALTER TABLE finance_manager.debt ALTER COLUMN category_name SET NOT NULL;

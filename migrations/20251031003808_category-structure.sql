CREATE TABLE finance_manager.debt_category (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    identification TEXT NOT NULL
);

CREATE UNIQUE INDEX idx_debt_category_identification ON finance_manager.debt_category(identification);
CREATE INDEX idx_debt_category_name ON finance_manager.debt_category(name);

ALTER TABLE finance_manager.debt ADD COLUMN category_id UUID REFERENCES finance_manager.debt_category(id);

CREATE INDEX idx_debt_category_id ON finance_manager.debt(category_id);
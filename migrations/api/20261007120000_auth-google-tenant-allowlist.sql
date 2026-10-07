-- Replace password-based auth with Google sign-in gated by an allowlist.
--
-- auth.tenant         who owns the data (the `client_id` used by the finance modules)
-- auth.allowed_users  emails authorized to sign in, each bound to a tenant and a role
-- auth.users          Google identities, created on first sign-in of an allowed email

CREATE TABLE auth.tenant (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP
);

-- Tenants are the existing clients, keeping their ids so finance data stays linked.
INSERT INTO auth.tenant (id, name, created_at, updated_at)
SELECT client_id, description, created_at, updated_at
FROM finance_manager.client_information;

CREATE TABLE auth.allowed_users (
    id UUID PRIMARY KEY,
    email VARCHAR(255) NOT NULL UNIQUE CHECK (email = LOWER(email)),
    tenant_id UUID NOT NULL REFERENCES auth.tenant(id),
    role VARCHAR(20) NOT NULL DEFAULT 'MEMBER' CHECK (role IN ('ADMIN', 'MEMBER')),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP
);

CREATE INDEX idx_allowed_users_tenant_id ON auth.allowed_users(tenant_id);

-- Every password-based user becomes an allowed email of its former client.
INSERT INTO auth.allowed_users (id, email, tenant_id, created_at)
SELECT gen_random_uuid(), LOWER(email), client_id, CURRENT_TIMESTAMP
FROM auth.users;

DROP TABLE auth.users;

-- Deleting an allowed_users row revokes access: the linked identity goes with it.
CREATE TABLE auth.users (
    id UUID PRIMARY KEY,
    allowed_user_id UUID NOT NULL UNIQUE REFERENCES auth.allowed_users(id) ON DELETE CASCADE,
    google_sub VARCHAR(255) NOT NULL UNIQUE,
    email VARCHAR(255) NOT NULL,
    name VARCHAR(255) NOT NULL,
    avatar_url TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP,
    last_login_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

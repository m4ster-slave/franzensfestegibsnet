CREATE TABLE IF NOT EXISTS auth_attempts (
    id SERIAL PRIMARY KEY,
    action VARCHAR(32) NOT NULL,
    ip_address VARCHAR(45) NOT NULL,
    identifier_hash VARCHAR(64),
    success BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT auth_attempts_action_check CHECK (action IN ('login', 'register'))
);

CREATE INDEX IF NOT EXISTS idx_auth_attempts_action_ip_created_at
    ON auth_attempts(action, ip_address, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_auth_attempts_login_identifier_created_at
    ON auth_attempts(action, ip_address, identifier_hash, created_at DESC)
    WHERE identifier_hash IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_posts_author_created_at
    ON posts(author_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_comments_author_created_at
    ON comments(author_id, created_at DESC);

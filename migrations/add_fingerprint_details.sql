DROP TABLE IF EXISTS fingerprint_posts;

CREATE TABLE fingerprint_details (
    fingerprint VARCHAR(64) PRIMARY KEY,
    ip_address VARCHAR(45) NOT NULL,
    user_agent VARCHAR(255) NOT NULL,
    last_post_timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    post_count INTEGER NOT NULL DEFAULT 1,
    UNIQUE(ip_address, user_agent)
);

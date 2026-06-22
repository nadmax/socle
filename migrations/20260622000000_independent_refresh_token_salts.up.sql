ALTER TABLE refresh_tokens ADD COLUMN lookup_key TEXT NOT NULL;
CREATE UNIQUE INDEX idx_refresh_tokens_lookup_key ON refresh_tokens(lookup_key);

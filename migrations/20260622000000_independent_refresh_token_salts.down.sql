DROP INDEX IF EXISTS idx_refresh_tokens_lookup_key;
ALTER TABLE refresh_tokens DROP COLUMN lookup_key;

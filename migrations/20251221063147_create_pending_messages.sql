CREATE TABLE pending_messages (
                                  id BIGSERIAL PRIMARY KEY,
                                  recipient_pubkey BYTEA NOT NULL,
                                  sender_pubkey BYTEA NOT NULL,
                                  encrypted_content BYTEA NOT NULL,
                                  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_pending_recipient ON pending_messages(recipient_pubkey);

CREATE OR REPLACE FUNCTION cleanup_old_pending_messages()
RETURNS void AS $$
BEGIN
DELETE FROM pending_messages
WHERE created_at < NOW() - INTERVAL '30 days';
END;
$$ LANGUAGE plpgsql;
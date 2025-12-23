ALTER TABLE pending_messages
    ADD COLUMN sender_enc_pubkey BYTEA;

DELETE FROM pending_messages WHERE sender_enc_pubkey IS NULL;

ALTER TABLE pending_messages
    ALTER COLUMN sender_enc_pubkey SET NOT NULL;
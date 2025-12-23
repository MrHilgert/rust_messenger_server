ALTER TABLE users RENAME COLUMN display_name TO first_name;

UPDATE users SET first_name = 'User' WHERE first_name IS NULL;

ALTER TABLE users ALTER COLUMN first_name SET NOT NULL;
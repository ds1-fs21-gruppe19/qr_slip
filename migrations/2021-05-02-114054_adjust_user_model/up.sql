ALTER TABLE qr_user ADD COLUMN name VARCHAR(255) NOT NULL DEFAULT '';

UPDATE qr_user SET name = coalesce(concat_ws(' ', first_name, last_name), '');

ALTER TABLE qr_user ALTER COLUMN name DROP DEFAULT;

ALTER TABLE qr_user DROP COLUMN first_name;
ALTER TABLE qr_user DROP COLUMN last_name;

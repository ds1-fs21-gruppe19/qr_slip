ALTER TABLE qr_user ADD COLUMN first_name VARCHAR(255);
ALTER TABLE qr_user ADD COLUMN last_name VARCHAR(255);

UPDATE qr_user SET first_name = name;

ALTER TABLE qr_user DROP COLUMN name;

CREATE TABLE qr_user(
    pk SERIAL PRIMARY KEY,
    first_name VARCHAR(255) NOT NULL,
    last_name VARCHAR(255) NOT NULL,
    address VARCHAR(255) NOT NULL,
    zip_code VARCHAR(255) NOT NULL,
    city VARCHAR(255) NOT NULL,
    iban VARCHAR(255) NOT NULL,
    country VARCHAR(255) NOT NULL
);

CREATE TABLE principal(
    pk SERIAL PRIMARY KEY,
    user_name VARCHAR(255) UNIQUE NOT NULL,
    password VARCHAR(255) NOT NULL,
    fk_user INTEGER REFERENCES qr_user(pk) NOT NULL
);
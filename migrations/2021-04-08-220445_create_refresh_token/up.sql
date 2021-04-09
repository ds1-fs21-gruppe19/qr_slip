CREATE TABLE refresh_token(
    pk SERIAL PRIMARY KEY,
    uuid UUID UNIQUE NOT NULL,
    expiry TIMESTAMP WITH TIME ZONE NOT NULL,
    invalidated BOOLEAN NOT NULL,
    fk_principal INTEGER REFERENCES principal(pk) NOT NULL
);
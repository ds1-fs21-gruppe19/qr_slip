# The backend for the qr_slip project

## Setup
 * For Diesel to be able to connect to the postgres database, the `DATABASE_URL` environment variable must be set,
   e.g. `DATABASE_URL=postgres://username:password@localhost/qr_slip`.
 * To generate JWT tokens the `JWT_SECRET` environment variable must be set.

These properties can be set locally in the .env file in the project directory for development.

To run schema migrations or create the initial database schema, run `diesel migration run`. When going live on production
it makes sense to enable running migrations on startup, but for development I'll stick to manual migrations for now.

To compile the project install the latest stable version of rust using [rustup](https://rustup.rs/), then run
`cargo build` to compile debug binaries or run `cargo build --release` to compile release binaries.

## Run
The binary can be executed by running `cargo run --release` in this directory.

## Endpoints

### `/login`
Generates and returns a JWT token for a given principal user_name. The request is expected to have a JSON that can be
deserialized to the following struct:
```rust
pub struct LoginRequest {
    pub user_name: String,
    pub password: String,
}
```
If the principal with the provided user_name does not exist or the hashed password does not match the endpoint returns
the following JSON and a 403 status code:
```json
{
    "message": "invalid credentials",
    "status": "403 Forbidden"
}
```

If the user_name exists and hashing the provided password matches the password on the DB, the server returns a token
which the client can use for the `Authorization: Bearer $token` header field for future requests.
```json
{
    "token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzUxMiJ9.eyJleHAiOjE2MTYyMDgxNTcsInN1YiI6InJvYmluZnJpZWRsaSJ9.XE8y4eKDGekeZptdqraTgqWkZsV8UVAuHjKUj2oY8zHptM1cqRr-Nwqkq6ecAjNe6Oo3uRPK8YALJCXGhTUDPw"
}
```

### `/register`
The register endpoint enables creating a new user and principal (login). The request is expected to have a JSON body that
can be deserialized to the following struct:
```rust
pub struct UserRegistration {
    pub first_name: String,
    pub last_name: String,
    pub address: String,
    pub zip_code: String,
    pub city: String,
    pub iban: String,
    pub country: String,
    pub user_name: String,
    pub password: String,
}
```
If the user_name for the principal is already taken, the server responds with the following JSON and a 400 status code:
```json
{
    "message": "There already exists a principal with the given identifier: 'my_user_name'",
    "status": "400 Bad Request"
}
```
If the request succeeded and the user and principal have been created the server simply responds with a 200 code.
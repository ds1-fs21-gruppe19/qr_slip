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
POST request.

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
POST request.

The register endpoint enables creating a new user and principal (login). The request is expected to have a JSON body that
can be deserialized to the following struct:
```rust
pub struct UserRegistration {
   pub first_name: Option<String>,
   pub last_name: Option<String>,
   pub address: String,
   pub zip_code: String,
   pub city: String,
   pub iban: String,
   pub country: String,
   pub user_name: String,
   pub password: String,
}
```
Note that first_name and last_name are optional fields.

If the user_name for the principal is already taken, the server responds with the following JSON and a 400 status code:
```json
{
    "message": "There already exists a principal with the given identifier: 'my_user_name'",
    "status": "400 Bad Request"
}
```
If the request succeeded and the user and principal have been created the server simply responds with a 200 code.

### `/create-user`
POST request.

Create a new User for the currently signed in principal. Requires an authorization header containing a JWT in the form
of "Bearer TOKEN" and expects a JSON body that can be deserialized to the following struct:
```rust
pub struct CreateUser {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub address: String,
    pub zip_code: String,
    pub city: String,
    pub iban: String,
    pub country: String,
}
```
Note that first_name and last_name are optional fields.

Simply returns a 200 if the operation was successful.

As any request that requires a login it returns a 401 when missing the authorization header or a 400 if the authorization
header is not formatted correctly.

### `/users`
GET request.

Returns all users created by the currently logged in principal. Requires an authorization header containing a JWT in the form
of "Bearer TOKEN".

As any request that requires a login it returns a 401 when missing the authorization header or a 400 if the authorization
header is not formatted correctly.

Example response:
```json
[
    {
        "pk": 2,
        "first_name": "Test",
        "last_name": "User",
        "address": "Downing Street 10",
        "zip_code": "SW1",
        "city": "London",
        "iban": "testiban",
        "country": "UK"
    },
    {
        "pk": 5,
        "first_name": null,
        "last_name": null,
        "address": "Thomas More St",
        "zip_code": "E1W 1YW",
        "city": "London",
        "iban": "iban",
        "country": "UK"
    }
]
```

### `/delete-users`
DELETE request.

Deletes all users where the principal matches the currently logged in principal and the primary key is included in the request.
Returns a json containing all users that have been deleted.

For example `/delete-users/6,8,9` might return this:
```json
[
    {
        "pk": 6,
        "first_name": null,
        "last_name": null,
        "address": "todel",
        "zip_code": "E1W 1YW",
        "city": "London",
        "iban": "iban",
        "country": "CH"
    },
    {
        "pk": 8,
        "first_name": null,
        "last_name": null,
        "address": "todel",
        "zip_code": "E1W 1YW",
        "city": "London",
        "iban": "iban",
        "country": "CH"
    },
    {
        "pk": 9,
        "first_name": null,
        "last_name": null,
        "address": "todel",
        "zip_code": "E1W 1YW",
        "city": "London",
        "iban": "iban",
        "country": "CH"
    }
]
```

As any request that requires a login it returns a 401 when missing the authorization header or a 400 if the authorization
header is not formatted correctly.
# The backend for the qr_slip project

## Setup with Docker

* run `docker-compose up --build` from root directory.

Docker-compose will create:
1. postgres container and run the sql/db.sql file to add all tables and relations.
2. rust container (base on debian `rust:latest`).

After that you should be able to send requests to `localhost:80`.

## Setup

* For Diesel to be able to connect to the postgres database, the `DATABASE_URL` environment variable must be set,
  e.g. `DATABASE_URL=postgres://username:password@localhost/qr_slip`.
* To generate JWT tokens the `JWT_SECRET` environment variable must be set.

These properties can be set locally in the .env file in the project directory for development.

To run schema migrations or create the initial database schema, run `diesel migration run`. When using the `auto_migration`
feature, migrations are executed at startup automatically, which should be the case when running the service in production,
see the run chapter.

To compile the project install the latest stable version of rust using [rustup](https://rustup.rs/), then run
`cargo build` to compile debug binaries or run `cargo build --release` to compile release binaries.

## Run

The binary can be executed by running `cargo run --release` in this directory. Running the debug binaries using
`cargo run` enables additional logging messages (all loggers set to level DEBUG, whereas the logger qr_slip::api,
which logs api requests, is set to WARN and other loggers are set to INFO when using release binaries).

When running in production, the feature `auto_migration` should be enabled so that migrations run at startup automatically
using `cargo run --release --features auto_migration`.

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
which the client can use for the `Authorization: Bearer $token` header field for future requests and the time until
the token expires in seconds. Additionally, the server sets the `refresh_token` cookie, which can be used to get a
new access token using the `/refresh-login` endpoint, this token is valid for 24 hours.

```json
{
    "token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzUxMiJ9.eyJleHAiOjE2MTc5Mjc5ODYsInN1YiI6InJvYmluZnJpZWRsaSJ9.3VRAuVSjkcZzB8wPVfi79JwOWq6g0fLx7Gd6uW4fiWmHTDKqblmR6HnVL_M5kUuOuKBYZ9qB2BMh_9kTiolDXA",
    "expiration_secs": 900
}
```

### `/refresh-login`

POST request.

Uses the refresh token stored in the `refresh_token` HttpOnly cookie to refresh the login of the associated principal
by returning a new JWT (same response as the `/login` endpoint) and updating the `refresh_token` cookie.

If the refresh_token is invalid, either because it does not exist, has been invalidated or is expired (older than
24 hours), the server responds with the following json and a 401 status code:

```json
{
    "message": "The provided refresh token is invalid",
    "status": "401 Unauthorized"
}
```

If the refresh_token is valid the server responds with the same response as the `/login` endpoint.

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

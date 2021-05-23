# The backend for the qr_slip project

## Setup with Docker

1. To run qr_slip via docker-compose, first clone the [Front-End](https://github.com/ds1-fs21-gruppe19/Front-end.git) and [qr_slip](https://github.com/ds1-fs21-gruppe19/qr_slip.git) into the same directory. e.g. `/home/qr_slip` and `/home/Front-End`
2. Then create a `config.env` file to `qr_slip/config.env`
3. Add the following variables & keys to the .env file. Please note to set the same values for `POSTGRES_USER` and `POSTGRES_PASSWORD` also to the `DATABASE_URL`.
  * `POSTGRES_USER=_someUser_`
  * `POSTGRES_PASSWORD=_somePassword_`
  * `POSTGRES_DB=qr_slip`
  * `JWT_SECRET=_someRandomUnsigned64_`
  * `DATABASE_URL=postgres://_someUser_:_somePassword_@db:5432/qr_slip`
4. For the first time run `docker-compose up --build` from root qr_slip root directory. Afterwards `docker-compose up` should be suficient.

Docker-compose will create:

1. postgres container.
2. Two rust containers (base on debian `rust:1.52.0`).
3. Frontend container (node:16-alpine3.13)
4. Loadbalancing container (nginx:alpine)

After that you should be able to request the frontend in your browser over `localhost:8000`.

The `docker-compose.yml` file starts two rust containers. Only the first container starts with feature auto_migration (as the Dockerfile does).
For the secound conatiner the yml-file overwrites the start-command without auto_migration.

## Setup

* For Diesel to be able to connect to the postgres database, the `DATABASE_URL` environment variable must be set,
  e.g. `DATABASE_URL=postgres://username:password@localhost/qr_slip`.
* To generate JWT tokens the `JWT_SECRET` environment variable must be set.

The environment variable `USE_PY_QR_GENERATOR` may be set to a boolean to toggle usage of the rq_generator.py script to
generate QR codes as an alternative to native QR code generation. This defaults to false but may be enabled in development
as using the python script simplifies experimenting with changes.

The environment variable `PDF_WORKER_POOL_SIZE` may be set to specify the number of processes in a pool used to convert
html to pdf via wkhtmltopdf. If 0 or not set, a single thread spawned by the main process is used to execute wkhtmltopdf
instead. This restriction exists because wkhtmltopdf can only be initialised once per process and only used by one thread.
Note that the process pool is only supported on macOS and Linux, on Windows and other platforms qr_slip always uses the
single worker thread.

These properties can be set locally in the .env file in the project directory for development.

To run schema migrations or create the initial database schema, run `diesel migration run`. When using the `auto_migration`
feature, migrations are executed at startup automatically, which should be the case when running the service in production,
see the run chapter.

Compiling requires [wkhtmltopdf](https://wkhtmltopdf.org/downloads.html) to be installed. On Arch / Manjaro Linux installing
the wkhtmltopdf package from the community repo should suffice, on Debian based distros the included wkhtmltopdf package
does not seem to contain the library, so you might want to download and install the .deb package* provided by wkhtmltopdf.
Additionally, you might need to install python3.8-dev and libpq-dev. On macOS, downloading and installing the .pkg from
the website should suffice. On Windows, download the installer and install wkhtmltopdf to `C:\Program Files\wkhtmltopdf`,
the build script build.rs adds the linker argument for the lib directory on that platform, and make sure that `C:\Program Files\wkhtmltopdf\bin`
has been added to the path so that the dll can be found at runtime.

*for example, on ubuntu 20.04 run

```bash
wget https://github.com/wkhtmltopdf/packaging/releases/download/0.12.6-1/wkhtmltox_0.12.6-1.focal_amd64.deb
sudo apt install ./wkhtmltox_0.12.6-1.focal_amd64.deb
```

To compile the project install the latest stable version of rust using [rustup](https://rustup.rs/), then run
`cargo build` to compile debug binaries or run `cargo build --release` to compile release binaries.

## Run

The binary can be executed by running `cargo run --release` in this directory. Running the debug binaries using
`cargo run` enables additional logging messages (all loggers set to level DEBUG, whereas the logger qr_slip::api,
which logs api requests, is set to WARN and other loggers are set to INFO when using release binaries) and enables the dbg
endpoints.

When running in production, the feature `auto_migration` should be enabled so that migrations run at startup automatically
using `cargo run --release --features auto_migration`.

## Development Environment

The recommended environment for working with the qr_slip codebase is VSCode with the rust-analyzer plugin or Intellij / CLion
with the Rust plugin.

## Documentation

The documentation can the rendered as an HTML site and opened in a browser using `cargo doc --open`.

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
    pub name: String,
    pub address: String,
    pub zip_code: String,
    pub city: String,
    pub iban: String,
    pub country: String,
    pub user_name: String,
    pub password: String,
}
```

Name is either the full name of a natural person or the name of a company.

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

Create a new User for the currently signed-in principal. Requires an authorization header containing a JWT in the form
of "Bearer TOKEN" and expects a JSON body that can be deserialized to the following struct:

```rust
pub struct CreateUser {
    pub name: String,
    pub address: String,
    pub zip_code: String,
    pub city: String,
    pub iban: String,
    pub country: String,
}
```

Name is either the full name of a natural person or the name of a company.

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
        "name": "Test User",
        "address": "Downing Street 10",
        "zip_code": "SW1",
        "city": "London",
        "iban": "testiban",
        "country": "UK"
    },
    {
        "pk": 3,
        "name": "Reynholm Industries",
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

Deletes all users where the principal matches the currently logged-in principal, and the primary key is included in the request.
Returns a json containing all users that have been deleted. Invalid primary keys that either do not exist or describe
entities that do not belong to the current principal are ignored.

For example `/delete-users/8,9,10,11` might return this if pk 9 does not exist and pk 10 does not belong to the current principal:

```json
[
  {
    "pk": 8,
    "name": "todel",
    "address": "todel",
    "zip_code": "E1W 1YW",
    "city": "London",
    "iban": "iban",
    "country": "UK"
  },
  {
    "pk": 11,
    "name": "todel2",
    "address": "todel2",
    "zip_code": "E1W 1YW",
    "city": "London",
    "iban": "iban",
    "country": "UK"
  }
]
```

As any request that requires a login it returns a 401 when missing the authorization header or a 400 if the authorization
header is not formatted correctly.

### `/generate-slip`

POST request.

Generates a PDF file where each page is a qr-slip created for a deserialized QrData element provided by the sequence of
JSON objects in the request body.

This request does not require any authentication as all user data is provided in the request. It is expected that the client
provides user data selected by the user from a `/users` request or user data that the user entered manually.

Each object provided in the sequence of JSON objects in the body must be able to be deserialized to the following struct:

```rust
pub struct QrData {
    creditor_iban: String,
    creditor_name: String,
    creditor_address: String,
    creditor_zip_code: String,
    creditor_city: String,
    creditor_country: String,
    debtor_name: String,
    debtor_address: String,
    debtor_zip_code: String,
    debtor_city: String,
    debtor_country: String,
    amount: String,
    currency: String,
    reference_type: String,
    reference_number: Option<String>,
    additional_information: Option<String>,
}
```

The `reference_type` must be one of the following items:

* QRR, which must be used if the `creditor_iban` is a QR-IBAN and requires that `reference_number` is set to a 27 digit numerical value
* SCOR, which must be used if the `creditor_iban` is an IBAN and `reference_number` is set (in that case the
   `reference_number` must be a 5 - 25 digit alphanumerical value)
* NON, which must be used if the `reference_number` is not set or empty

These conditions and length restrictions for each field are verified and the endpoint returns a 400 BAD REQUEST on violation.

See the official [specification](https://www.paymentstandards.ch/dam/downloads/ig-qr-bill-de.pdf).

The endpoint returns the PDF file in the body and the header Content-Type set to application/pdf.

### `/dbg-qr-pdf` (debug binaries only)

POST request.

Generates a PDF file containing a qr slip on each page. This request functions the same as `/generate-slip` and expects
the same input but saves the created PDF file to a new file in the `tmp/` directory. This request is meant to be used in
development and is only available when running the debug binaries.

### `dbg-qr-html` (debug binaries only)

POST request.

Generates an HTML file containing all qr slips which would later be used to generate the PDF file. This endpoint functions
the same as `/dbg-qr-pdf` only that it does not perform the step that would create the PDF file and saves an HTML file
instead.

### `dbg-qr-svg` (debug binaries only)

POST request.

Generates a qr code that encodes the provided data in the format specified by the [six documentation](https://www.paymentstandards.ch/dam/downloads/ig-qr-bill-de.pdf).

This endpoint functions the same as `/dbg-qr-html` only that it does not perform the step that would create the HTML file
and saves an SVG file containing the QR code instead. Also, this endpoint expects a single JSON object, not a sequence.

# The backend for the qr_slip project

## Database setup
For Diesel to be able to connect to the postgres database, the `DATABASE_URL` environment variable must be set,
e.g. `DATABASE_URL=postgres://username:password@localhost/qr_slip`.

## Run
The binary can be executed by running `cargo run --release` in this directory.
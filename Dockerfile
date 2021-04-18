FROM rust:latest

WORKDIR /
COPY /. ./

RUN cargo install diesel_cli --no-default-features --features postgres
RUN cargo build --release

CMD [ "cargo", "run", "--release" ]


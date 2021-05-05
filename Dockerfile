FROM rust:latest

WORKDIR /
COPY /. ./

RUN cargo install diesel_cli --no-default-features --features postgres
RUN cargo build --release

 # Debug
#CMD [ "cargo", "run" ]
 #Relase
#CMD [ "cargo", "run", "--release" ]
# Relase with auto migration
#CMD [ "cargo", "run", "--release", "--features auto_migration" ]

# Debug with auto migration
CMD [ "cargo", "run", "--release" ]

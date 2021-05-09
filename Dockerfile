FROM rust:1.52.0

WORKDIR /
COPY /. ./
RUN rm ./build.rs


#Install wkhtmltopdf with prerequiments
RUN apt update
RUN apt install xfonts-base -f -y
RUN apt install xfonts-75dpi -f -y
RUN apt install python-dev -f -y
RUN apt install libpq-dev -f -y
RUN wget https://github.com/wkhtmltopdf/packaging/releases/download/0.12.6-1/wkhtmltox_0.12.6-1.buster_amd64.deb
RUN dpkg -i ./wkhtmltox_0.12.6-1.buster_amd64.deb
RUN rm wkhtmltox_0.12.6-1.buster_amd64.deb

#Python 3.7
RUN apt install build-essential
RUN curl -O https://www.python.org/ftp/python/3.7.3/Python-3.7.3.tar.xz
RUN tar -xf Python-3.7.3.tar.xz
RUN ./Python-3.7.3/configure --enable-optimizations
RUN make altinstall


RUN cargo install diesel_cli --no-default-features --features postgres

RUN cargo build --release

CMD [ "cargo", "run", "--release", "--features", "auto_migration"] 
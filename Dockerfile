FROM rust:1.52.0

WORKDIR /opt/qr_slip
COPY /. ./

#Install wkhtmltopdf with prerequiments
RUN apt update
RUN apt install xfonts-base -f -y
RUN apt install xfonts-75dpi -f -y
RUN apt install python3.7-dev -f -y
RUN apt install libpq-dev -f -y
RUN wget https://github.com/wkhtmltopdf/packaging/releases/download/0.12.6-1/wkhtmltox_0.12.6-1.buster_amd64.deb
RUN dpkg -i ./wkhtmltox_0.12.6-1.buster_amd64.deb
RUN rm wkhtmltox_0.12.6-1.buster_amd64.deb

RUN cargo build --release

CMD [ "cargo", "run", "--release", "--features", "auto_migration"]
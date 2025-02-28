FROM rust:1-slim

COPY . /src
WORKDIR /src

RUN cargo build --release

CMD ["/src/target/release/wadio", "--music-path", "/music", "--api"]
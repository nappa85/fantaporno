FROM rust:bullseye AS builder

WORKDIR /app

RUN git clone https://github.com/nappa85/fantaporno.git && cd fantaporno && cargo build --release

FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y libsqlite3-0 ca-certificates libfontconfig1 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/fantaporno/target/release/fantaporno /usr/local/bin/fantaporno

RUN chmod +x /usr/local/bin/fantaporno

WORKDIR /usr/local/bin/

CMD fantaporno
FROM rust:bullseye AS builder
WORKDIR /usr/src/timer-api
COPY . .
RUN cargo install --path .

FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/timer-api /usr/local/bin/timer-api
CMD ["timer-api"]
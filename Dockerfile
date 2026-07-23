FROM rust:1.88-bookworm AS build
WORKDIR /src
COPY . .
RUN cargo build -p honknet-server --release --locked
FROM debian:bookworm-slim
RUN useradd -r -u 10001 honknet
COPY --from=build /src/target/release/honknet-server /usr/local/bin/honknet-server
USER honknet
EXPOSE 3015/udp
ENTRYPOINT ["honknet-server","--listen","0.0.0.0:3015"]

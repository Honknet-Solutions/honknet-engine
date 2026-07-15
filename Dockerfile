# syntax=docker/dockerfile:1.7
FROM node:22-bookworm-slim AS web-build
WORKDIR /src
COPY package.json package-lock.json ./
COPY apps ./apps
COPY packages ./packages
COPY tools ./tools
COPY examples ./examples
COPY templates ./templates
RUN npm ci --no-audit --no-fund
COPY . .
RUN npm run validate && npm run typecheck && npm test && npm run build

FROM rust:1.85.1-bookworm AS rust-build
WORKDIR /src
COPY Cargo.toml rust-toolchain.toml ./
COPY apps/server ./apps/server
COPY crates ./crates
RUN cargo generate-lockfile && cargo build --workspace --release --locked

FROM node:22-bookworm-slim AS runtime
ENV NODE_ENV=production \
    RUST_LOG=info \
    HONKNET_CONFIG=/opt/honknet/engine.toml
WORKDIR /opt/honknet
RUN useradd --system --uid 10001 --create-home --home-dir /opt/honknet honknet
COPY --from=rust-build /src/target/release/honknet-server /usr/local/bin/honknet-server
COPY --from=web-build /src/apps/script-host/dist ./apps/script-host/dist
COPY --from=web-build /src/examples/minimal-game/server/dist ./examples/minimal-game/server/dist
COPY --from=web-build /src/apps/client/dist ./apps/client/dist
COPY --from=web-build /src/tools/studio/dist ./tools/studio/dist
COPY --from=web-build /src/examples/minimal-game/content ./examples/minimal-game/content
COPY --from=web-build /src/examples/minimal-game/maps ./examples/minimal-game/maps
COPY --from=web-build /src/examples/minimal-game/resources ./examples/minimal-game/resources
COPY --from=web-build /src/examples/minimal-game/localization ./examples/minimal-game/localization
COPY deploy/engine.docker.toml ./engine.toml
RUN mkdir -p data/saves && chown -R honknet:honknet /opt/honknet
USER honknet
EXPOSE 3015 3016
VOLUME ["/opt/honknet/data/saves"]
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD node -e "fetch('http://127.0.0.1:3016/healthz').then(r=>{if(!r.ok)process.exit(1)}).catch(()=>process.exit(1))"
ENTRYPOINT ["honknet-server"]

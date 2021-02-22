FROM rust as planner
WORKDIR polygon-data-relay
# We only pay the installation cost once, 
# it will be cached from the second build onwards
# To ensure a reproducible build consider pinning 
# the cargo-chef version with `--version X.X.X`
RUN cargo install cargo-chef 

COPY . .

RUN cargo chef prepare  --recipe-path recipe.json

FROM rust as cacher
WORKDIR polygon-data-relay
RUN cargo install cargo-chef
COPY --from=planner /polygon-data-relay/recipe.json recipe.json
RUN --mount=type=ssh cargo chef cook --release --recipe-path recipe.json

FROM rust as builder
WORKDIR polygon-data-relay
COPY . .
# Copy over the cached dependencies
COPY --from=cacher /polygon-data-relay/target target
COPY --from=cacher /usr/local/cargo /usr/local/cargo
RUN --mount=type=ssh cargo build --release --bin polygon-data-relay

FROM debian:buster-slim as runtime
WORKDIR polygon-data-relay
COPY --from=builder /polygon-data-relay/target/release/polygon-data-relay /usr/local/bin
ENV RUST_LOG=polygon-data-relay=debug
ENTRYPOINT ["/usr/local/bin/polygon-data-relay"]

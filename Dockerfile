# syntax=docker.io/docker/dockerfile:1

FROM docker.io/rust:slim-trixie AS builder

WORKDIR /app

RUN --mount=source=src,target=src \
  --mount=source=Cargo.toml,target=Cargo.toml \
  --mount=source=Cargo.lock,target=Cargo.lock \
  --mount=type=cache,target=/app/target,sharing=locked \
  --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
  cargo build --features mimalloc --release --locked \
  && cp target/release/proxy-spider .


FROM docker.io/debian:trixie-slim AS final

WORKDIR /app

ARG \
  UID=1000 \
  GID=1000

RUN (getent group "${GID}" || groupadd --gid "${GID}" app) \
  && useradd --gid "${GID}" --no-log-init --create-home --uid "${UID}" app \
  && mkdir -p /home/app/.cache/proxy_spider \
  && chown ${UID}:${GID} /home/app/.cache/proxy_spider

COPY --from=builder --chown=${UID}:${GID} --link /app/proxy-spider .

USER app

CMD ["/app/proxy-spider"]

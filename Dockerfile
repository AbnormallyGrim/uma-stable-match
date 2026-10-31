# syntax=docker/dockerfile:1

# Builds the web page.
FROM node:26-bookworm-slim AS frontend
WORKDIR /build/frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci
COPY frontend/ ./
RUN npm run build

# Builds the server. aws-lc-sys, used by rustls, compiles C code with the system C compiler.
FROM rust:1-bookworm AS server
RUN apt-get update \
    && apt-get install -y --no-install-recommends build-essential \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY backend/ backend/
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build --release --locked -p uma-stable-match-server \
    && cp target/release/uma-stable-match-server /usr/local/bin/

# Runs both: no shell or package manager, CA certificates included, non-root.
FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=server /usr/local/bin/uma-stable-match-server /app/uma-stable-match-server
COPY --from=frontend /build/frontend/dist /app/frontend
# The project website, served only when SITE_HOST and SITE_DIR are set.
COPY site/ /app/site
ENV HOST=0.0.0.0 \
    PORT=8080 \
    FRONTEND_DIR=/app/frontend
EXPOSE 8080
USER nonroot
ENTRYPOINT ["/app/uma-stable-match-server"]

# Stage 1: Build Rust binary
FROM rust:latest AS builder

RUN rustup default nightly

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
# Cache dependencies
RUN mkdir src && echo "fn main(){}" > src/main.rs && cargo build --release 2>/dev/null || true && rm -rf src
COPY src/ src/
RUN cargo build --release

# Stage 2: Python runtime
FROM python:3.12-slim-bookworm

COPY --from=builder /build/target/release/agent-search /usr/local/bin/

COPY requirements.txt /app/
RUN pip install --no-cache-dir -r /app/requirements.txt

COPY agent/ /app/agent/
COPY web/ /app/web/
COPY entrypoint.sh /app/

RUN chmod +x /app/entrypoint.sh

WORKDIR /app
ENV AGENT_CORPUS=/corpus
EXPOSE 8080

ENTRYPOINT ["/app/entrypoint.sh"]

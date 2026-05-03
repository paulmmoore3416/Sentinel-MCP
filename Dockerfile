# Build stage
FROM rust:1.75-slim-bookworm as builder
WORKDIR /usr/src/app
COPY . .
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim
WORKDIR /app
# Install dependencies for diagnostic tools
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    jq \
    openssl \
    iproute2 \
    iptables \
    procps \
    sysstat \
    dmesg \
    && rm -rf /var/lib/apt/lists/*

# Install kubectl for Kubernetes-related MCP tools
RUN curl -LO "https://dl.k8s.io/release/$(curl -L -s https://dl.k8s.io/release/stable.txt)/bin/linux/amd64/kubectl" && \
    install -o root -g root -m 0755 kubectl /usr/local/bin/kubectl && \
    rm kubectl

COPY --from=builder /usr/src/app/target/release/sentinel-mcp /usr/local/bin/sentinel-mcp
COPY --from=builder /usr/src/app/config.example.yaml /app/config.yaml

EXPOSE 3000
CMD ["sentinel-mcp", "start", "--port", "3000"]

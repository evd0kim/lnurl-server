# Stage 1: Build
FROM acinq/phoenixd:latest as builder

USER root

# Install build dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    build-essential \
    pkg-config \
    libssl-dev \
    cmake \
    clang \
    libgflags-dev \
    libsnappy-dev \
    zlib1g-dev \
    libbz2-dev \
    liblz4-dev \
    libzstd-dev \
    libc++-dev \
    && rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain nightly
ENV PATH="/root/.cargo/bin:/usr/local/bin:$PATH"

WORKDIR /app

# Copy source code
COPY src /app/src
COPY Cargo.* /app/

# Build the application with phoenixd feature
RUN cargo build --release --features phoenixd

# Stage 2: Runtime
FROM acinq/phoenixd:latest

USER root

# Install required packages
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    supervisor \
    && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

# Copy the compiled lnurl-server binary from builder
COPY --from=builder /app/target/release/lnurl-server /usr/local/bin/lnurl-server
RUN chmod +x /usr/local/bin/lnurl-server

# Create directories for supervisor and data
RUN mkdir -p /var/log/supervisor /etc/supervisor/conf.d /app/data

# Copy supervisor configurations
COPY docker/supervisord.conf /etc/supervisor/supervisord.conf
COPY docker/phoenixd.conf /etc/supervisor/conf.d/phoenixd.conf
COPY docker/lnurl-server.conf /etc/supervisor/conf.d/lnurl-server.conf

# Copy entrypoint script
COPY docker/entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

# Health check - verify lnurl-server is responding
HEALTHCHECK --interval=10s --timeout=3s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:3000/health-check || exit 1

EXPOSE 3000

ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
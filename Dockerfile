FROM rust:latest AS builder

WORKDIR /usr/src/franzensfestegibsnet/

# Install Diesel CLI, libsmbclient-dev, and other dependencies
RUN cargo install diesel_cli --no-default-features --features postgres

# Copy the application code
COPY . .

# Build the Rust application
RUN cargo build --release

# Stage 2: Prepare the runtime environment
FROM debian:bookworm-slim

# Install wkhtmltopdf and other necessary packages
# RUN apt-get update && \
#     apt-get install -y \

# Copy the compiled application from the build stage
COPY --from=builder /usr/src/franzensfestegibsnet/target/release/franzensfestegibsnet /usr/local/bin/franzensfestegibsnet

# Copy Diesel CLI from the build stage
COPY --from=builder /usr/local/cargo/bin/diesel /usr/local/bin/diesel

# Copy other directories
# COPY Rocket.prod.toml Rocket.toml
COPY templates templates
COPY articles articles 
COPY public /usr/src/franzensfestegibsnet/public

# Set the default command
CMD ["franzensfestegibsnet"]

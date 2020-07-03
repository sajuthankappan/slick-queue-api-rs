# Use the official Rust image.
# https://hub.docker.com/_/rust
FROM rust:1.44.0 as builder
WORKDIR /usr/src

# Create a dummy project and build the app's dependencies.
# If the Cargo.toml or Cargo.lock files have not changed,
# we can use the docker build cache and skip these (typically slow) steps.
RUN USER=root cargo new api
WORKDIR /usr/src/api
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release

# Copy the source and build the application.
COPY src ./src
RUN touch ./src/main.rs
#RUN cargo install --path .
RUN cargo build --release

# our final base
FROM debian:buster-slim

RUN apt-get update
RUN apt-get install -y openssl
#RUN apt-get install -y libcurl4

# copy the build artifact from the build stage
#COPY --from=builder /usr/local/cargo/bin/page-score-api /server
COPY --from=builder /usr/src/api/target/release/slick-queue-api /server

# Run the web service on container startup.
CMD ["/server"]

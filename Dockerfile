
# Use the latest stable version of Rust as the base image to build the project

FROM rust:1.85-bookworm AS build
WORKDIR /opt/backend

# Init the repository

COPY ./Cargo.toml ./Cargo.lock ./.gitmodules ./
COPY ./src ./src
COPY ./migrations ./migrations
COPY ./.git ./.git
RUN git submodule update --init --recursive

# Install the required dependencies

RUN apt update && apt install --no-install-recommends -y racket

# Build the project

RUN cargo build --release

# Create a new image with only the necessary files to run the application

FROM debian:bookworm AS runtime
WORKDIR /opt/backend
RUN groupadd -r backend && useradd -r -g backend backend
USER backend
COPY --from=build /opt/backend/target/release/backend backend
ENTRYPOINT [ "./backend" ]
# Use the stable version of Rust as the base image
FROM rust:1.85-bookworm

# Install the required dependencies
RUN apt update && apt install --no-install-recommends -y racket

# Set the application directory
WORKDIR /opt/backend

# Init the repository
COPY .git .git
RUN git restore . && git submodule update --init --recursive

# Build the project
RUN cargo build -r

CMD [ "cargo", "run", "-r" ]
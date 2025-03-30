# Use the stable version of Rust as the base image
FROM rust:1.85-bookworm

# Install the required dependencies
RUN apt update && apt install --no-install-recommends -y racket

# Create a non-root user to run the application
RUN groupadd -r backend && useradd -r -g backend backend
USER backend

# Set the application directory
WORKDIR /opt/backend

# Init the repository
COPY --chown=backend:backend .git .git
RUN git restore .
RUN git submodule update --init --recursive

# Build the project
RUN cargo build -r

CMD [ "cargo", "run", "-r" ]
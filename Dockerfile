FROM ubuntu:22.04
WORKDIR /back-end
EXPOSE 3000

# Install dependencies

RUN apt-get update && apt-get install -y \
    racket \
    rustup \
    && apt-get clean
RUN rustup install stable

# Clone the repository

RUN git clone https://github.com/se-clavier/back-end.git .
RUN git submodule update --init --recursive

# Prepare the environment

RUN racket api/rust.rkt < api/api.rkt > api/src/lib.rs

# Build the project

RUN cargo build --release

ENTRYPOINT [ "cargo", "run", "--release" ]
FROM ubuntu:22.04 as build
WORKDIR /back-end

# Install dependencies

RUN apt-get update && apt-get --no-install-recommends install -y \
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

FROM ubuntu:22.04 as runtime
USER back-end
EXPOSE 3000
COPY --from=build target/release/back-end /back-end
ENTRYPOINT [ "/back-end" ]
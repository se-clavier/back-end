
# Use the latest stable version of Rust as the base image to build the project

FROM rust:1.85-bookworm AS build
WORKDIR /opt/back-end

# Clone the repository

COPY . .
RUN git submodule update --init --recursive

# Build the project

RUN make -C api prepare
RUN make -C api src/lib.rs
RUN cargo build --release

# Create a new image with only the necessary files to run the application

FROM debian:12 AS runtime
USER back-end
EXPOSE 3000
COPY --from=build target/release/back-end /back-end
ENTRYPOINT [ "/back-end" ]
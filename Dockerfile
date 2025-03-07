
# Use the latest stable version of Rust as the base image to build the project

FROM rust:1.85-bookworm AS build
WORKDIR /opt/back-end

# Clone the repository

COPY Cargo.* .gitmodules ./
COPY src src
COPY .git .git
RUN git submodule update --init --recursive

# Build the project

RUN make -C api prepare
RUN make -C api src/lib.rs
RUN cargo build --release

# Create a new image with only the necessary files to run the application

FROM debian:bookworm AS runtime
RUN groupadd -r back-end && useradd -r -g back-end back-end
USER back-end
EXPOSE 3000
COPY --from=build /opt/back-end/target/release/back-end /back-end
ENTRYPOINT [ "/back-end" ]
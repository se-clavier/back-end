FROM rust:1.85 AS build
WORKDIR /back-end

# Clone the repository

RUN git clone https://github.com/se-clavier/back-end.git .
RUN git submodule update --init --recursive

# Build the project

RUN make -C api prepare
RUN make -C api src/lib.rs
RUN cargo build --release

FROM debian:12 AS runtime
USER back-end
EXPOSE 3000
COPY --from=build target/release/back-end /back-end
ENTRYPOINT [ "/back-end" ]
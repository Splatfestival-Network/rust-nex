FROM rust:alpine AS builder

WORKDIR /app

COPY . .

RUN apk add --no-cache protobuf-dev git musl-dev lld

RUN git submodule update --init --recursive

RUN RUSTFLAGS="-C relocation-model=static -C linker=ld.lld" cargo build --profile prod --target x86_64-unknown-linux-musl

FROM scratch AS final

# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/x86_64-unknown-linux-musl/prod/splatoon-server-rust /splatoon-server-rust

# Command to run the application
ENTRYPOINT ["/splatoon-server-rust"]

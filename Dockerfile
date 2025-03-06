FROM rust:1.85 AS builder

WORKDIR /app

COPY . .

RUN apt-get update && apt-get install protobuf-compiler -y

RUN cargo build --release

FROM rust:1.85 AS final

WORKDIR /app

# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/release/splatoon-server-rust /app/splatoon-server-rust

# Set executable permissions
RUN chmod +x /app/splatoon-server-rust

# Command to run the application
CMD ["/app/splatoon-server-rust"]

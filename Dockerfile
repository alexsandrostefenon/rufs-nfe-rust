FROM rust-runtime
COPY ./webapp /app/webapp
COPY ./sql /app/sql
COPY ./pkg/*.wasm /app/pkg/
COPY ./pkg/*.js /app/pkg/
COPY ./target/release/rufs-nfe-rust /app/
EXPOSE 8080
ENTRYPOINT ["/app/rufs-nfe-rust"]
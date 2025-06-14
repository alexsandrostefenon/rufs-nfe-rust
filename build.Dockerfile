FROM localhost:5000/rust:latest
ARG working_dir
WORKDIR $working_dir
RUN cargo install wasm-pack
RUN cargo build -r
RUN wasm-pack build --target web

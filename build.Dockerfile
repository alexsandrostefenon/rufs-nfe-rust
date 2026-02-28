FROM localhost:5000/rust:latest
RUN apt-get update -y \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends firebird-dev \
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*
ARG working_dir
WORKDIR $working_dir
RUN cargo install wasm-pack
RUN cargo build -r
RUN wasm-pack build --target web

# rufs-nfe-rust

Restful Utilities for Full Stack - Brazilian NFE WebApp

You need Rust + wasm-bindgen installed and PostgreSql server already running with your database.

Requires Rust version >= 1.63

Requires browser with support to dynamic ES6 modules (tested with Chrome versions >= 64)

## Containerized application

Open terminal and clone this repository with `git clone https://github.com/alexsandrostefenon/rufs-nfe-rust`.

Edit "rufs-nfe-rust/.env" with your personal data.

### Setup build and testing environment
`
cargo install cargo-version-upgrade;\
cd rufs-nfe-rust &&\
podman-compose up -d standalone_firefox &&\
podman-compose up -d postgres &&\
export $(cat .env | xargs) &&\
PGHOST=localhost &&\
psql postgresql://postgres:$POSTGRES_PASSWORD@$PGHOST:$PGPORT/template1 -c "CREATE USER $PGUSER WITH CREATEDB LOGIN PASSWORD '$PGPASSWORD'" &&\
psql postgresql://postgres:$POSTGRES_PASSWORD@$PGHOST:$PGPORT/template1 -c "CREATE DATABASE rufs_nfe WITH OWNER $PGUSER" &&\
podman build -t selenium-side-runner -f selenium-side-runner.Dockerfile &&\
podman build -t rust-runtime -f runtime.Dockerfile &&\
podman build -v $PWD/../rufs-base-rust:$PWD/../rufs-base-rust -v $PWD:$PWD -t rust-build --build-arg working_dir=$PWD -f build.Dockerfile &&\
echo "Setup of containerized environment is done !";
`

### Run

Build container image and run service :
`
#cargo-version-upgrade patch;\
podman-compose down rufs-crud-rust;\
./build.sh &&\
podman-compose up -d rufs-crud-rust;
`

### Tests :
`
./tests/tests.sh;
`

## Web application

In EcmaScript2017 compliance browser open url

`http://localhost:8081`

For custom service configuration or user edition, use user 'admin' with password 'admin'.

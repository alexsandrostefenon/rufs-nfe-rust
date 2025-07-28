# rufs-nfe-rust

Restful Utilities for Full Stack - Brazilian NFE WebApp

You need podman (or docker) and podman-compose (or docker-compose).

Podman is my preferency because run in user-space.

Requires browser with support to dynamic ES6 modules (tested with Chrome versions >= 64)

## Containerized application

Open terminal and clone this repository with :
```
git clone https://github.com/alexsandrostefenon/rufs-nfe-rust
```

### Setup build and testing environment
```
cd rufs-nfe-rust
```
Edit ".env" with your personal data and execute:
```
podman-compose up -d standalone_firefox &&
podman-compose up -d postgres &&
export $(cat .env | xargs) &&
PGHOST=localhost &&
psql postgresql://postgres:$POSTGRES_PASSWORD@$PGHOST:$PGPORT/template1 -c "CREATE USER $PGUSER WITH CREATEDB LOGIN PASSWORD '$PGPASSWORD'" &&
psql postgresql://postgres:$POSTGRES_PASSWORD@$PGHOST:$PGPORT/template1 -c "CREATE DATABASE rufs_nfe WITH OWNER $PGUSER" &&
podman build -t selenium-side-runner -f selenium-side-runner.Dockerfile &&
podman build -t rust-runtime -f runtime.Dockerfile &&
podman build -v $PWD/../rufs-base-rust:$PWD/../rufs-base-rust -v $PWD:$PWD -t rust-build --build-arg working_dir=$PWD -f build.Dockerfile &&
echo "Setup of containerized environment is done !";
```

### Run

To build container image and run service:
```
podman-compose down rufs-nfe;
./build.sh &&
podman-compose up -d rufs-nfe;
```

### Tests :
```
./tests/tests.sh;
```

## Web application

In EcmaScript2017 compliance browser open url

`http://localhost:8081`

For custom service configuration or user edition, use user 'admin' with password of first login.

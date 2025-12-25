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
podman-compose up -d registry &&
sleep 5 &&
podman-compose up -d postgres &&
sleep 5 &&
podman-compose up -d redis &&
sleep 5 &&
podman-compose up -d nginx &&
sleep 15 &&
export $(cat .env | grep -v 'POSTGRES_INITDB_ARGS' | xargs) &&
podman exec -it postgres psql postgresql://postgres:$POSTGRES_PASSWORD@localhost:$PGPORT/template1 -c "CREATE USER $PGUSER WITH CREATEDB LOGIN PASSWORD '$PGPASSWORD'" &&
podman exec -it postgres psql postgresql://postgres:$POSTGRES_PASSWORD@localhost:$PGPORT/template1 -c "CREATE DATABASE rufs_nfe WITH OWNER $PGUSER" &&
podman build -t selenium-side-runner -f selenium-side-runner.Dockerfile &&
podman build -t rust-runtime -f runtime.Dockerfile &&
podman build -v $PWD/../rufs-base-rust:$PWD/../rufs-base-rust -v $PWD:$PWD -t rust-build --build-arg working_dir=$PWD -f build.Dockerfile &&
echo "Setup of containerized environment is done !";
```

### Run

To build container image and run service:
```
podman-compose down rufs-nfe &&
podman-compose down nfe-import &&
./build.sh &&
podman-compose up -d rufs-nfe &&
podman-compose up -d nfe-import
```

### Tests :
```
./tests/tests.sh;
```

## Web application

In EcmaScript2017 compliance browser open url

`http://localhost:8080/nfe/`

For custom service configuration or user edition, use user 'admin' with password of first login.

Utils
https://portaldatransparencia.gov.br/pessoa-juridica/<cnpj>;
https://servicos.receita.fazenda.gov.br/servicos/cpf/consultasituacao/consultapublica.asp (<cpf> <data_nascimento>);
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
./cloud.sh --setup
```

### Run

To build container image and run service:
```
./cloud.sh --build --update
```

### Tests :
```
./cloud.sh --test;
```

## Web application

In EcmaScript2017 compliance browser open url

`http://localhost:8080/nfe/`

For custom service configuration or user edition, use user 'admin' with password of first login.

Utils
https://portaldatransparencia.gov.br/pessoa-juridica/<cnpj>;
https://servicos.receita.fazenda.gov.br/servicos/cpf/consultasituacao/consultapublica.asp (<cpf> <data_nascimento>);

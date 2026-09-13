# rufs-nfe-rust

Restful Utilities for Full Stack - Brazilian NFE WebApp

You need podman (or docker) and podman-compose (or docker-compose).

Podman is my preferency because run in user-space.

## Containerized application

Open terminal and clone this repository with :
```
git clone https://github.com/alexsandrostefenon/rufs-nfe-rust
```

### Setup build and testing environment
```
cd rufs-nfe-rust
```
Replace variables in ".env" with your customized data and execute:
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

The system is fully multi-tenant data (splited in postgres schemas). Each tenant have one group key id (like as brazlian CPNJ) with your dedicated user 'admin'.

At first acess, enter with your preferd login name and one secury password.

For users management, login with user 'admin' and same password of first login.

## TODO Features
https://portaldatransparencia.gov.br/pessoa-juridica/<cnpj>;
https://servicos.receita.fazenda.gov.br/servicos/cpf/consultasituacao/consultapublica.asp (<cpf> <data_nascimento>);

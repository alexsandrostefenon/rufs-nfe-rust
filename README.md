# rufs-nfe-rust

Restful Utilities for Full Stack - Brazilian NFE WebApp

You need Rust + wasm-bindgen installed and PostgreSql server already running with your database.

Requires Rust version >= 1.63

Requires browser with support to dynamic ES6 modules (tested with Chrome versions >= 64)

## First Step

Open terminal and clone this repository with `git clone https://github.com/alexsandrostefenon/rufs-nfe-rust`.

To download the required dependencies and build, then

`wasm-pack build --target web --dev` 

## Run Ecosystem

## PostgreSql setup

create database :

sudo su postgres;

or

su -c "su postgres";

export PGDATABASE=postgres;
psql -c "CREATE USER development WITH CREATEDB LOGIN PASSWORD '123456'";
psql -c 'CREATE DATABASE rufs_nfe_development WITH OWNER development';
exit;

Note, database "rufs_nfe_development" is only for testing purposes.

### Run Ecosystem

#Only to clean already existent configuration :


#Only to clean already existent testing data :
`
clear; \
rm *openapi-rufs_nfe-rust.json; \
PGHOST=localhost PGPORT=5432 PGUSER=development PGPASSWORD=123456 psql rufs_nfe_development -c "DROP DATABASE IF EXISTS rufs_nfe;" &&
PGHOST=localhost PGPORT=5432 PGUSER=development PGPASSWORD=123456 psql rufs_nfe_development -c "CREATE DATABASE rufs_nfe;" &&
`
#Execute rufs-proxy to load and start microservices :

#PGHOST=localhost PGPORT=5432 PGUSER=development PGPASSWORD=123456 PGDATABASE=rufs_nfe nodejs ./rufs-base-es6/proxy.js --add-modules ../rufs-nfe-es6/NfeMicroService.js;
PGHOST=localhost PGPORT=5432 PGUSER=development PGPASSWORD=123456 PGDATABASE=rufs_nfe nodejs --inspect ./rufs-nfe-es6/NfeMicroService.js;

## Web application

In EcmaScript2017 compliance browser open url

`http://localhost:8080`

For custom service configuration or user edition, use user 'admin' with password 'admin'.

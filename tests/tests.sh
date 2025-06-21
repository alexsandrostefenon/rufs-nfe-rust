#!/bin/sh
set -x
PS4=' $LINENO: '
set -e

export $(cat .env | xargs)

if [ "$1" = 'reset-container' ] || [ "$2" = 'reset-container' ]; then
    podman-compose down rufs-crud-rust
fi

if [ "$1" = 'drop-db' ] || [ "$2" = 'drop-db' ]; then
    rm -f $HOME/data/openapi-rufs_nfe.json
    PGHOST=localhost psql rufs_nfe_development -c "DROP DATABASE IF EXISTS rufs_nfe"
    PGHOST=localhost psql rufs_nfe_development -c "CREATE DATABASE rufs_nfe"
fi

if [ "$1" = 'reset-container' ] || [ "$2" = 'reset-container' ]; then
    podman-compose up -d rufs-crud-rust
fi

PGHOST=localhost psql rufs_nfe -c "DROP SCHEMA IF EXISTS rufs_customer_12345678901 CASCADE"
echo "Reseted testing data !"
mkdir -p ./tmp
podman run --rm -v $PWD:$PWD -w $PWD selenium-side-runner:latest selenium-side-runner -j '"--detectOpenHandles"' tests.side

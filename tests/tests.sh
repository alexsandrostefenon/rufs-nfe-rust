#!/bin/sh
set -x
PS4=' $LINENO: '
set -e

export $(cat .env | grep -v 'POSTGRES_INITDB_ARGS' | xargs)

podman-compose down selenium-side-runner
podman-compose down selenium-standalone-firefox
podman-compose down rufs-nfe
podman-compose down redis
#podman-compose down broker
podman-compose down nginx
podman-compose down postgres

podman-compose up -d postgres
sleep 7

if [ "$1" = 'drop-db' ]; then
    rm -f $HOME/data/openapi-rufs_nfe.json
    rm -f $HOME/data/rufs_customer_template.sql
    podman exec -it postgres psql postgresql://postgres:$POSTGRES_PASSWORD@localhost:$PGPORT/template1 -c "DROP DATABASE IF EXISTS rufs_nfe"
    podman exec -it postgres psql postgresql://postgres:$POSTGRES_PASSWORD@localhost:$PGPORT/template1 -c "CREATE DATABASE rufs_nfe WITH OWNER $PGUSER"
fi

podman exec -it postgres psql postgres://$PGUSER:$PGPASSWORD@localhost:$PGPORT/rufs_nfe -c "DROP SCHEMA IF EXISTS rufs_customer_12345678901 CASCADE"
echo "Reseted testing data !"

podman-compose up -d nginx
#podman-compose up -d broker
#podman exec -it broker /opt/kafka/bin/kafka-topics.sh --bootstrap-server localhost:9092 --create --topic nfe
podman-compose up -d redis
podman-compose up -d rufs-nfe
podman-compose up -d selenium-standalone-firefox
sleep 8
podman-compose up -d selenium-side-runner

mkdir -p ./tmp
rm -f ./tmp/*

podman-compose exec selenium-side-runner selenium-side-runner -j '"--detectOpenHandles"' tests.side

podman-compose down selenium-side-runner
podman-compose down selenium-standalone-firefox

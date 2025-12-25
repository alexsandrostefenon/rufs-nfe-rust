#!/bin/sh
set -x
PS4=' $LINENO: '
set -e

PARSED=$(getopt --options='ei:' --longoptions='db-drop,db-import:' --name "$0" -- "$@") || exit 2
eval set -- "$PARSED"

while true; do
  case "$1" in
  -e | --db-drop)
    db_drop='yes'
    shift
    ;;
  -i | --db-import)
    db_import="$2"
    shift 2
    ;;
  --)
    shift
    break
    ;;
  *)
    echo "Programming error"
    exit 3
    ;;
  esac
done

export $(cat .env | grep -v 'POSTGRES_INITDB_ARGS' | xargs)

podman-compose down selenium-side-runner
podman-compose down selenium-standalone-firefox
podman-compose down rufs-nfe
podman-compose down nfe-import
podman-compose down redis
#podman-compose down broker
podman-compose down nginx
podman-compose down postgres

podman-compose up -d postgres
sleep 5

if [ "$db_drop" = 'yes' ]; then
  rm -f $HOME/data/openapi-rufs_nfe.json
  rm -f $HOME/data/rufs_customer_template.sql
  podman exec -it postgres psql postgresql://postgres:$POSTGRES_PASSWORD@localhost:$PGPORT/template1 -c "DROP DATABASE IF EXISTS rufs_nfe"
  podman exec -it postgres psql postgresql://postgres:$POSTGRES_PASSWORD@localhost:$PGPORT/template1 -c "CREATE DATABASE rufs_nfe WITH OWNER $PGUSER"
fi

if [ "$db_import" != '' ]; then
  echo 'SET session_replication_role = replica;' > /tmp/tmp.sql
  gunzip -c $db_import >> /tmp/tmp.sql
  cat /tmp/tmp.sql | podman exec -i postgres psql -1 postgres://postgres:$POSTGRES_PASSWORD@localhost:$PGPORT/rufs_nfe

  if [ "$db_drop" = 'yes' ]; then
    echo '{"openapi": "3.0.3","info": {"title": "rufs-nfe","version": "1.0.17"},"paths": {},"components": {"schemas": {}}}' > $HOME/data/openapi-rufs_nfe.json
    podman exec postgres pg_dump -n rufs_customer_template --inserts postgres://$PGUSER:$PGPASSWORD@localhost:$PGPORT/rufs_nfe -f /app/data/rufs_customer_template.sql
  fi
fi

podman exec -it postgres psql postgres://$PGUSER:$PGPASSWORD@localhost:$PGPORT/rufs_nfe -c "DROP SCHEMA IF EXISTS rufs_customer_12345678901 CASCADE"
echo "Reseted testing data !"

podman-compose up -d nginx
#podman-compose up -d broker
#podman exec -it broker /opt/kafka/bin/kafka-topics.sh --bootstrap-server localhost:9092 --create --topic nfe
podman-compose up -d redis
podman-compose up -d nfe-import
podman-compose up -d rufs-nfe
podman-compose up -d selenium-standalone-firefox
sleep 8
podman-compose up -d selenium-side-runner

mkdir -p ./tmp
rm -f ./tmp/*

podman-compose exec selenium-side-runner selenium-side-runner -j '"--detectOpenHandles"' tests.side

podman-compose down selenium-side-runner
podman-compose down selenium-standalone-firefox

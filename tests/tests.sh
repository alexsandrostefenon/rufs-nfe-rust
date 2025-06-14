#tests.sh
#reset-data.sh
#podman-compose up -d postgres;
#podman-compose down rufs-crud-rust;
export $(cat .env | xargs)
#rm -f $HOME/data/openapi-rufs_nfe.json &&
#PGHOST=localhost psql rufs_nfe_development -c "DROP DATABASE IF EXISTS rufs_nfe" &&
#PGHOST=localhost psql rufs_nfe_development -c "CREATE DATABASE rufs_nfe" &&
PGHOST=localhost psql rufs_nfe -c "DROP SCHEMA IF EXISTS rufs_customer_12345678901 CASCADE" &&
echo "Reseted testing data !" &&
podman run --rm -v $PWD:$PWD -w $PWD selenium-side-runner:latest selenium-side-runner -j '"--detectOpenHandles"' tests.side
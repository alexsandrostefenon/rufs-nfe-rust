#!/bin/bash
set -x
PS4=' $LINENO: '
set -e

cmd_env='./.env'
cmd_target='release'

PARSED=$(getopt --options='ei:' --longoptions='env:,db-backup,db-drop,db-drop-tests-schemas,db-import:,install,setup,build,target:,test,deploy,update,scp-in:' --name "$0" -- "$@") || exit 2
eval set -- "$PARSED"

while true; do
  case "$1" in
  --env)
    cmd_env="$2"
    shift 2
    ;;
  --install)
    cmd_install='yes'
    shift
    ;;
  --setup)
    cmd_setup='yes'
    shift
    ;;
  --build)
    cmd_build='yes'
    shift
    ;;
  --target)
    cmd_target="$2"
    shift 2
    ;;
  -e | --db-drop)
    cmd_db_drop='yes'
    shift
    ;;
  --db-drop-tests-schemas)
    cmd_db_drop_tests_schemas='yes'
    shift
    ;;
  --db-backup)
    cmd_db_backup='yes'
    shift
    ;;
  -i | --db-import)
    cmd_import="$2"
    shift 2
    ;;
  --test)
    cmd_test='yes'
    shift
    ;;
  --deploy)
    cmd_deploy='yes'
    shift
    ;;
  --update)
    cmd_update='yes'
    shift
    ;;
  --scp-in)
    cmd_scp_in="$2"
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

source "$cmd_env"
docker_cli_local='podman'
tls_no_verify='--tls-verify=false'
compose_cli_local='podman-compose'
exec=''

function db_backup() {
  $exec $docker_cli_target exec postgres mkdir -p /app/data/backup/db
  backup_date=$(date '+%y%m%d-%H%M')
  backup_file="data/backup/db/$backup_date.sql.gz"
  $exec $docker_cli_target exec postgres pg_dump -Z6 --clean --if-exists --inserts postgres://$PGUSER:$PGPASSWORD@localhost:$PGPORT/rufs_nfe -f /app/$backup_file
  mkdir -p ./data/backup/db
  scp $ssh_connection_args $vps_user@$vps_ip:$backup_file ./$backup_file
}

function db_drop() {
  $compose_cli_local down rufs-nfe
  rm -f $HOME/data/openapi-rufs_nfe.json
  rm -f $HOME/data/rufs_customer_template.sql
  $docker_cli_local exec -it postgres psql postgresql://postgres:$POSTGRES_PASSWORD@localhost:$PGPORT/template1 -c "DROP DATABASE IF EXISTS rufs_nfe"
  $docker_cli_local exec -it postgres psql postgresql://postgres:$POSTGRES_PASSWORD@localhost:$PGPORT/template1 -c "CREATE DATABASE rufs_nfe WITH OWNER $PGUSER"
}

if [ "$vps_provider" != 'local' ]; then
  scp $ssh_connection_args cloud.sh scp://$vps_user@$vps_ip
  exec="ssh $ssh_connection_args $vps_user@$vps_ip"
fi

if ! command -v $docker_cli_local &>/dev/null; then
    docker_cli_local='docker'
    tls_no_verify=''

    if ! command -v $docker_cli_local &>/dev/null; then
        docker_cli_local='none'
    fi
fi

if ! command -v $compose_cli_local &>/dev/null; then
  compose_cli_local='docker-compose'

  if ! command -v $compose_cli_local &>/dev/null; then
    compose_cli_local='docker compose'
  fi
fi

if [ "$cmd_install" = 'yes' ]; then
    #https://photogabble.co.uk/tutorials/running-amazon-linux-2023-within-virtualbox/
    wget -c https://cdn.amazonlinux.com/al2023/os-images/2023.8.20250818.0/kvm/al2023-kvm-2023.8.20250818.0-kernel-6.1-x86_64.xfs.gpt.qcow2
    #nano meta-data
    #nano user-data
    mkisofs -output seed.iso -volid cidata -joliet -rock user-data meta-data;
    # TODO : configure network
    #qemu-system-x86_64 -name al2023 -accel kvm -cpu host -m 2048 -monitor stdio -k pt-br -rtc base=localtime -drive file=seed.iso,media=cdrom -drive file=al2023-kvm-2023.6.20241212.0-kernel-6.1-x86_64.xfs.gpt.qcow2
fi

if [ "$cmd_setup" = 'yes' ]; then
    if [ "$vps_provider" = 'aws' ]; then
        $exec sudo yum install -y docker postgresql16.x86_64
        $exec sudo usermod -aG docker $vps_user
        $exec sudo systemctl enable docker
        $exec sudo systemctl start docker
        $exec sudo curl -L https://github.com/docker/compose/releases/latest/download/docker-compose-linux-$(uname -m) -o /usr/bin/docker-compose
        $exec sudo chmod 755 /usr/bin/docker-compose
    fi

    if [ "$vps_provider" != 'local' ]; then
      scp $ssh_connection_args $cmd_env scp://$vps_user@$vps_ip:22/.env
      scp $ssh_connection_args -r ./nginx scp://$vps_user@$vps_ip
      scp $ssh_connection_args compose.yml scp://$vps_user@$vps_ip
    fi

    $exec mkdir -p data
    $exec $compose_cli_local up -d postgres redis registry
    sleep 15
    $exec $docker_cli_target exec -it postgres psql postgresql://postgres:$POSTGRES_PASSWORD@localhost:$PGPORT/template1 -c "CREATE USER $PGUSER WITH CREATEDB LOGIN PASSWORD '$PGPASSWORD'" &&
    $exec $docker_cli_target exec -it postgres psql postgresql://postgres:$POSTGRES_PASSWORD@localhost:$PGPORT/template1 -c "CREATE DATABASE rufs_nfe WITH OWNER $PGUSER" &&
    $exec mkdir -p ./etc/letsencrypt
    $exec mkdir -p ./var/lib/letsencrypt

    #if [ "$vps_provider" != 'local' ]; then
      #$exec $docker_cli_target run -it --rm --name certbot -p 80:80 -v "./etc/letsencrypt:/etc/letsencrypt" -v "./var/lib/letsencrypt:/var/lib/letsencrypt" certbot/certbot certonly;#select '1'
      ##$exec $docker_cli_target  run --rm --entrypoint=cat nginx /etc/nginx/nginx.conf > ./nginx/etc/nginx.conf
    #fi

    if [ "$vps_provider" = 'local' ]; then
      $docker_cli_target build -t selenium-side-runner -f selenium-side-runner.Dockerfile &&
      $docker_cli_target build -t rust-runtime -f runtime.Dockerfile &&
      $docker_cli_target build -v $PWD/../rufs-base-rust:$PWD/../rufs-base-rust -v $PWD:$PWD -t rust-build --build-arg working_dir=$PWD -f build.Dockerfile &&
      echo "Setup of containerized environment is done !";
    fi
fi

if [ "$cmd_build" != '' ]; then
  #$docker_cli_local pull --tls-verify=false localhost:5000/rust-runtime
  #$docker_cli_local pull --tls-verify=false localhost:5000/rust-build

  release_debug_server='--release'
  release_debug_client='--release'

  if [ "$cmd_target" = 'debug' ]; then
      release_debug_server=''
      release_debug_client='--dev'
  fi

  exec="$docker_cli_local run --rm -v $PWD/../rufs-base-rust:$PWD/../rufs-base-rust -v $PWD:$PWD -w $PWD -it rust-build"
  $exec cargo build $release_debug_server
  $exec wasm-pack build $release_debug_client --target web
  version=$($exec cargo pkgid 2>/dev/null | grep -oP '\d+\.\d+\.\d+')
  $docker_cli_local build -v $PWD:$PWD -t rufs-nfe-rust:$version ./
  $docker_cli_local tag rufs-nfe-rust:$version rufs-nfe-rust:latest
  echo "Build of containerized application image is done !"
fi

if [ "$cmd_db_drop_tests_schemas" = 'yes' ]; then
  $docker_cli_local exec -it postgres psql postgres://$PGUSER:$PGPASSWORD@localhost:$PGPORT/rufs_nfe -c "DROP SCHEMA IF EXISTS rufs_customer_12345678901 CASCADE"
fi

if [ "$cmd_test" = 'yes' ]; then
  $compose_cli_local down selenium-side-runner
  $compose_cli_local down selenium-standalone-firefox
  $compose_cli_local down rufs-nfe
  $compose_cli_local down nfe-import
  $compose_cli_local down redis
  #$compose_cli_local down broker
  $compose_cli_local down nginx
  $compose_cli_local down postgres

  $compose_cli_local up -d postgres
  sleep 5

  if [ "$cmd_db_drop" = 'yes' ]; then
    db_drop
  fi

  if [ "$cmd_import" != '' ]; then
    echo 'SET session_replication_role = replica;' > /tmp/tmp.sql
    gunzip -c $cmd_import >> /tmp/tmp.sql
    cat /tmp/tmp.sql | $docker_cli_local exec -i postgres psql -1 postgres://postgres:$POSTGRES_PASSWORD@localhost:$PGPORT/rufs_nfe

    if [ "$cmd_db_drop" = 'yes' ]; then
      echo '{"openapi": "3.0.3","info": {"title": "rufs-nfe","version": "1.0.21"},"paths": {},"components": {"schemas": {}}}' > $HOME/data/openapi-rufs_nfe.json
      $docker_cli_local exec postgres pg_dump -n rufs_customer_template --inserts postgres://$PGUSER:$PGPASSWORD@localhost:$PGPORT/rufs_nfe -f /app/data/rufs_customer_template.sql
    fi
  fi

  $docker_cli_local exec -it postgres psql postgres://$PGUSER:$PGPASSWORD@localhost:$PGPORT/rufs_nfe -c "DROP SCHEMA IF EXISTS rufs_customer_12345678901 CASCADE"
  echo "Reseted testing data !"
  $compose_cli_local up -d nginx
  #$compose_cli_local up -d broker
  #$docker_cli_local exec -it broker /opt/kafka/bin/kafka-topics.sh --bootstrap-server localhost:9092 --create --topic nfe
  $compose_cli_local up -d redis
  $compose_cli_local up -d nfe-import
  $compose_cli_local up -d rufs-nfe
  $compose_cli_local up -d selenium-standalone-firefox
  sleep 8
  $compose_cli_local up -d selenium-side-runner
  mkdir -p ./tmp
  rm -f ./tmp/*
  $compose_cli_local exec selenium-side-runner selenium-side-runner -j '"--bail 1 --detectOpenHandles"' tests.side
  $compose_cli_local down selenium-side-runner
  $compose_cli_local down selenium-standalone-firefox
fi

if [ "$cmd_deploy" = 'yes' ]; then
    exec_c="$docker_cli_local run --rm -v $PWD/../rufs-base-rust:$PWD/../rufs-base-rust -v $PWD:$PWD -w $PWD -it rust-build"
    tunel_port='5000'
    version=$($exec_c cargo pkgid 2>/dev/null | grep -oP '\d+\.\d+\.\d+')

    if [ "$vps_provider" != 'local' ]; then
      tunel_port='6000'
      ssh -fgNC $ssh_connection_args ssh://$vps_user@$vps_ip -L $tunel_port:127.0.0.1:5000 -oServerAliveInterval=60
    fi

    $docker_cli_local tag rufs-nfe-rust:$version localhost:$tunel_port/rufs-nfe-rust:$version
    $docker_cli_local tag rufs-nfe-rust:$version localhost:$tunel_port/rufs-nfe-rust:latest
    $docker_cli_local push $tls_no_verify localhost:$tunel_port/rufs-nfe-rust:$version
    $docker_cli_local push $tls_no_verify localhost:$tunel_port/rufs-nfe-rust:latest
fi

if [ "$cmd_db_backup" = 'yes' ]; then
  #$exec $compose_cli_target down nginx
  #$exec $compose_cli_target down nfe-import
  #$exec $compose_cli_target down rufs-nfe
  db_backup
  #$exec $compose_cli_target up -d rufs-nfe
  #$exec $compose_cli_target up -d nfe-import
  #$exec $compose_cli_target up -d nginx
fi

if [ "$cmd_update" = 'yes' ]; then
    $exec $compose_cli_target down nginx
    $exec $compose_cli_target down nfe-import
    $exec $compose_cli_target down rufs-nfe

    if [ "$vps_provider" != 'local' ]; then
      db_backup
      $exec $docker_cli_target pull localhost:5000/rufs-nfe-rust:latest
    fi

    $exec $compose_cli_target up -d rufs-nfe
    $exec $compose_cli_target up -d nfe-import

    if [ "$vps_provider" != 'local' ]; then
      $exec $docker_cli_target run --rm --name certbot -p 80:80 -v "./etc/letsencrypt:/etc/letsencrypt" -v "./var/lib/letsencrypt:/var/lib/letsencrypt" certbot/certbot renew
    fi

    $exec $compose_cli_target up -d nginx
    $exec $compose_cli_target logs -f rufs-nfe nfe-import nginx
fi

if [ "$cmd_test" != 'yes' ] && [ "$cmd_db_drop" = 'yes' ]; then
  db_drop
fi

if [ "$cmd_test" != 'yes' ] && [ "$cmd_import" != '' ]; then
  if [ "$cmd_import" = 'remote' ]; then
    ##psql -c 'select nfe_id from rufs_customer_80803792034.request_nfe order by nfe_id' | sed 's/\s//g' > /tmp/nfe_id_db_sorted.txt
    ##grep -hoP '\d{10,}\s\d{10,}' tests/nfg-20*.csv | sort | uniq | sed 's/\s//g' > /tmp/nfe_id_csv_sorted.txt
    ##psql -c 'drop schema if exists rufs_customer_12345678901 cascade';
    ##psql -c 'ALTER SCHEMA rufs_customer_80803792034 RENAME TO rufs_customer_12345678901';
    db_backup
    cmd_import=$backup_file
    source "./.env"
    db_drop
  fi

  echo 'SET session_replication_role = replica;' > /tmp/tmp.sql
  gunzip -c $cmd_import >> /tmp/tmp.sql
  cat /tmp/tmp.sql | $docker_cli_local exec -i postgres psql -1 postgres://postgres:$POSTGRES_PASSWORD@localhost:$PGPORT/rufs_nfe
  echo '{"openapi": "3.0.3","info": {"title": "rufs-nfe","version": "1.0.21"},"paths": {},"components": {"schemas": {}}}' > $HOME/data/openapi-rufs_nfe.json
  $docker_cli_local exec postgres pg_dump -n rufs_customer_template --inserts postgres://$PGUSER:$PGPASSWORD@localhost:$PGPORT/rufs_nfe -f /app/data/rufs_customer_template.sql
fi

if [ "$cmd_scp_in" != '' ]; then
  scp $ssh_connection_args scp://$vps_user@$vps_ip/$cmd_scp_in $cmd_scp_in
fi

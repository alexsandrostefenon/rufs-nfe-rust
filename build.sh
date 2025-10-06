#!/bin/bash
set -x
PS4=' $LINENO: '
set -e

#podman pull --tls-verify=false localhost:5000/rust-runtime
#podman pull --tls-verify=false localhost:5000/rust-build

release_debug_server='--release'
release_debug_client='--release'

if [ "$1" = 'debug' ]; then
    release_debug_server=''
    release_debug_client='--dev'
fi

exec="podman run --rm -v $PWD/../rufs-base-rust:$PWD/../rufs-base-rust -v $PWD:$PWD -w $PWD -it rust-build"

$exec cargo build $release_debug_server
$exec wasm-pack build $release_debug_client --target web
version=$($exec cargo pkgid 2>/dev/null | grep -oP '\d+\.\d+\.\d+')
podman build -v $PWD:$PWD -t rufs-nfe-rust:$version ./
podman tag rufs-nfe-rust:$version rufs-nfe-rust:latest
echo "Build of containerized application image is done !"
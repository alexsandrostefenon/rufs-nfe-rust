#!/bin/bash
set -x
PS4=' $LINENO: '
set -e

#build.sh
#To use debug profile, replace "--release" by "--dev" :
#podman pull --tls-verify=false localhost:5000/rust-runtime
#podman pull --tls-verify=false localhost:5000/rust-build
podman run --rm -v $PWD/../rufs-base-rust:$PWD/../rufs-base-rust -v $PWD:$PWD -w $PWD -it rust-build cargo build --release
podman run --rm -v $PWD/../rufs-base-rust:$PWD/../rufs-base-rust -v $PWD:$PWD -w $PWD -it rust-build wasm-pack build --release --target web
version=$(cargo pkgid 2>/dev/null | cut -d "#" -f2)
podman build -v $PWD:$PWD -t rufs-nfe-rust:$version ./
podman tag rufs-nfe-rust:$version rufs-nfe-rust:latest
echo "Build of containerized application image is done !"
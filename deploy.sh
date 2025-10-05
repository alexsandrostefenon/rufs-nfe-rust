#!/bin/bash
set -x
PS4=' $LINENO: '
set -e

tunel_port='6000'
tls_no_verify='--tls-verify=false'
version=$(cargo pkgid 2>/dev/null | cut -d "#" -f2)
ssh -fgNC $ssh_connection_args ssh://ec2-user@$aws_ip -L $tunel_port:127.0.0.1:5000 -oServerAliveInterval=60
podman tag rufs-nfe-rust:$version localhost:$tunel_port/rufs-nfe-rust:$version
podman tag rufs-nfe-rust:$version localhost:$tunel_port/rufs-nfe-rust:latest
podman push $tls_no_verify localhost:$tunel_port/rufs-nfe-rust:$version
podman push $tls_no_verify localhost:$tunel_port/rufs-nfe-rust:latest

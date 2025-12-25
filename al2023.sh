#!/bin/bash
set -x
PS4=' $LINENO: '
set -e

exec="ssh $ssh_connection_args ec2-user@$aws_ip"

if [ "$1" = 'install' ]; then
    #https://photogabble.co.uk/tutorials/running-amazon-linux-2023-within-virtualbox/
    wget -c https://cdn.amazonlinux.com/al2023/os-images/2023.8.20250818.0/kvm/al2023-kvm-2023.8.20250818.0-kernel-6.1-x86_64.xfs.gpt.qcow2
    #nano meta-data
    #nano user-data
    mkisofs -output seed.iso -volid cidata -joliet -rock user-data meta-data;
    # TODO : configure network
    #qemu-system-x86_64 -name al2023 -accel kvm -cpu host -m 2048 -monitor stdio -k pt-br -rtc base=localtime -drive file=seed.iso,media=cdrom -drive file=al2023-kvm-2023.6.20241212.0-kernel-6.1-x86_64.xfs.gpt.qcow2
elif [ "$1" = 'setup' ]; then
    scp $ssh_connection_args al2023.sh .env compose.yml scp://ec2-user@$aws_ip
    $exec sudo yum install -y docker postgresql16.x86_64
    $exec sudo usermod -aG docker ec2-user
    $exec sudo systemctl enable docker
    $exec sudo systemctl start docker
    $exec sudo curl -L https://github.com/docker/compose/releases/latest/download/docker-compose-linux-$(uname -m) -o /usr/bin/docker-compose
    $exec sudo chmod 755 /usr/bin/docker-compose
elif [ "$1" = 'update' ]; then
    $exec docker-compose down nfe-import
    $exec docker-compose down rufs-nfe
    $exec docker exec postgres mkdir -p /app/data/backup
    backup=$(date '+%y%m%d-%H%M')
    $exec docker exec postgres pg_dump -Z6 --inserts postgres://$PGUSER:$PGPASSWORD@localhost:$PGPORT/rufs_nfe -f /app/data/backup/$backup.sql.gz
    scp $ssh_connection_args ec2-user@$aws_ip:data/backup/$backup.sql.gz ./
    $exec docker pull localhost:5000/rufs-nfe-rust:latest
    $exec docker-compose up -d rufs-nfe
    $exec docker-compose up -d nfe-import
    $exec ./letsencrypt.sh
    $exec docker-compose logs -f rufs-nfe nfe-import
fi

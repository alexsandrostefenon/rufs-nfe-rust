#!/bin/bash
set -x
PS4=' $LINENO: '
set -e

ssh_connection_args=''

if [ "$2" != '' ]; then
    #<remote_server_name>.pem
    ssh_connection_args="-i $2";
fi

if [ "$1" = 'install' ]; then
    #https://photogabble.co.uk/tutorials/running-amazon-linux-2023-within-virtualbox/
    wget -c https://cdn.amazonlinux.com/al2023/os-images/2023.6.20241212.0/vmware/al2023-vmware_esx-2023.6.20241212.0-kernel-6.1-x86_64.xfs.gpt.ova
    tar -xvf al2023-vmware_esx-2023.6.20241212.0-kernel-6.1-x86_64.xfs.gpt.ova
    nano meta-data
    nano user-data
    mkisofs -output seed.iso -volid cidata -joliet -rock user-data meta-data;
    VBoxManage clonemedium al2023-vmware_esx-2023.6.20241212.0-kernel-6.1-x86_64.xfs.gpt-disk1.vmdk ~/al2023-vmware_esx-2023.6.20241212.0-kernel-6.1-x86_64.xfs.gpt-disk1.vdi --format VDI;
    VBoxManage ~/al2023-vmware_esx-2023.6.20241212.0-kernel-6.1-x86_64.xfs.gpt-disk1.vdi;
elif [ "$1" = 'setup' ]; then
    sudo yum install -y docker postgresql16.x86_64
    sudo usermod -aG docker ec2-user
    sudo systemctl enable docker
    sudo systemctl start docker
    sudo curl -L https://github.com/docker/compose/releases/latest/download/docker-compose-linux-$(uname -m) -o /usr/bin/docker-compose
    sudo chmod 755 /usr/bin/docker-compose
    #scp $ssh_connection_args al2023.sh .env compose.yml scp://ec2-user@$aws_ip;
elif [ "$1" = 'deploy' ]; then
    exec="ssh $ssh_connection_args ec2-user@$aws_ip"
    $exec docker-compose down rufs-crud-rust
    $exec docker pull localhost:5000/rufs-nfe-rust:latest
    $exec docker-compose up -d rufs-crud-rust
    #$exec ~/letsencrypt.sh
    $exec docker-compose logs -f rufs-crud-rust
fi

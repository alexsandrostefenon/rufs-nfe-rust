#!/bin/bash

#docker rm -f ec2-user-rufs-crud-rust-1 ec2-user-nginx-1
docker down rufs-crud-rust
docker down nginx

if [ "$1" = 'install' ]; then
    docker run -it --rm --name certbot -p 80:80 -v "./etc/letsencrypt:/etc/letsencrypt" -v "./var/lib/letsencrypt:/var/lib/letsencrypt" certbot/certbot certonly
    #Certificate is saved at: /etc/letsencrypt/live/xxx.com/fullchain.pem
    #Key is saved at:         /etc/letsencrypt/live/xxx.com/privkey.pem
    docker run --rm --entrypoint=cat nginx /etc/nginx/nginx.conf > ./nginx/etc/nginx.conf
else
    docker run -it --rm --name certbot -p 80:80 -v "./etc/letsencrypt:/etc/letsencrypt" -v "./var/lib/letsencrypt:/var/lib/letsencrypt" certbot/certbot renew
fi

docker-compose up -d nginx rufs-crud-rust

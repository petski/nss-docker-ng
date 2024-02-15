# Introduction

`nss-docker-ng` is a plugin for the GNU Name Service Switch (NSS) functionality of the GNU C Library (glibc) for finding Docker containers by their ID or name.

The container names are searched in a virtual domain name `.docker`.

Install it, then try:

```
$ docker run -d --name my-app my/app
04e8b7ccf6215a17064ec0d15d4235d5c62c461bc26a681c8ee64344bb0dc2df

$ getent hosts my-app.docker
172.17.0.4      my-app.docker

$ ping my-app.docker
PING test.docker (172.17.0.4) 56(84) bytes of data.
64 bytes from 172.17.0.4: icmp_seq=1 ttl=64 time=0.171 ms
```

# Installation instructions

```
cargo build --release && \
 sudo install -m 0755 -d /opt/nss-docker-ng/ && \
 sudo install -m 0644 target/release/libnss_docker_ng.so /opt/nss-docker-ng/libnss_docker_ng.so.2 && \
 echo '/opt/nss-docker-ng' | sudo tee /etc/ld.so.conf.d/nss-docker-ng.conf > /dev/null && \
 sudo /sbin/ldconfig
```

Add the `docker_ng` service to the `hosts:`-line in `/etc/nsswitch.conf`. For example:

```
hosts: files docker_ng dns
```

# Development

```
cargo build && \
 sudo install -m 0755 -d /opt/nss-docker-ng/ && \
 sudo install -m 0644 target/debug/libnss_docker_ng.so /opt/nss-docker-ng/libnss_docker_ng.so.2 && \
 echo '/opt/nss-docker-ng' | sudo tee /etc/ld.so.conf.d/nss-docker-ng.conf > /dev/null && \
 sudo /sbin/ldconfig
```

# Useful links

* https://github.com/dex4er/nss-docker/
* https://docs.rs/docker-api/latest/docker_api/
* https://docs.docker.com/engine/api/

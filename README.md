[![QC](https://github.com/petski/nss-docker-ng/actions/workflows/qc.yml/badge.svg)](https://github.com/petski/nss-docker-ng/actions/workflows/qc.yml)
[![crates.io](https://img.shields.io/crates/v/nss-docker-ng.svg)](https://crates.io/crates/nss-docker-ng)

# Introduction

`nss-docker-ng` is a NSS plugin for finding Docker containers by their ID or name.

The container names are searched in a virtual domain name `.docker`.

Install it, then try:

```
$ docker run --name my-app -d hashicorp/http-echo -listen=:80 -text="✅ it works!"
04e8b7ccf6215a17064ec0d15d4235d5c62c461bc26a681c8ee64344bb0dc2df

$ getent hosts my-app.docker
172.17.0.4      my-app.docker 04e8b7ccf621.docker

$ curl http://my-app.docker
✅ it works!

$ ping my-app.docker
PING test.docker (172.17.0.4) 56(84) bytes of data.
64 bytes from 172.17.0.4: icmp_seq=1 ttl=64 time=0.171 ms

$ docker stop my-app
```

# Subdomain / wildcard behavior

You can have `nss-docker-ng` respond to all subdomains for a container by setting the label `.com.github.petski.nss-docker-ng.container-subdomains-allowed` to `true`, `True` or `1`.

```
$ docker run --name my-app-with-subdomains -d --label=".com.github.petski.nss-docker-ng.container-subdomains-allowed=true" hashicorp/http-echo -listen=:80 -text="✅ it works!"
670b3dc0aa2761d0fa150180e1cc1769d5e7e5a3332c12562197c7a782ed8a94

$ getent hosts my-app-with-subdomains.docker
172.29.0.2      my-app-with-subdomains.docker 670b3dc0aa27.docker

$ getent hosts foo.my-app-with-subdomains.docker
172.29.0.2      my-app-with-subdomains.docker 670b3dc0aa27.docker foo.my-app-with-subdomains.docker

$ docker stop my-app-with_subdomains
```

# Installation instructions

## Binary install

```
DESTDIR="/usr/local/lib/nss-docker-ng/" && \
 sudo --preserve-env=DESTDIR install -m 0755 -d "$DESTDIR" && \
 curl -sL 'https://github.com/petski/nss-docker-ng/releases/latest/download/libnss_docker_ng.so' -o - | sudo --preserve-env=DESTDIR tee "${DESTDIR}/libnss_docker_ng.so" > /dev/null && \
 echo "${DESTDIR}" | sudo tee /etc/ld.so.conf.d/nss-docker-ng.conf > /dev/null && \
 sudo /sbin/ldconfig
```

Then, add the `docker_ng` service to the `hosts:`-line in `/etc/nsswitch.conf`. For example: `hosts: files docker_ng dns`

## Ubuntu 22.04 "jammy" and higher

```
sudo add-apt-repository ppa:petski/ubuntu/nss-docker-ng && \
 sudo apt install nss-docker-ng
```

## From source

You'll need at least `git`, `cargo` and `patchelf`.

```
git clone https://github.com/petski/nss-docker-ng.git
cd nss-docker-ng
cargo build --release && \
 patchelf --set-soname libnss_docker_ng.so.2 target/release/libnss_docker_ng.so && \
 DESTDIR="/usr/local/lib/nss-docker-ng/" && \
 sudo --preserve-env=DESTDIR install -m 0755 -d "$DESTDIR" && \
 sudo --preserve-env=DESTDIR install -m 0644 target/release/libnss_docker_ng.so "${DESTDIR}/libnss_docker_ng.so" && \
 echo "${DESTDIR}" | sudo tee /etc/ld.so.conf.d/nss-docker-ng.conf > /dev/null && \
 sudo /sbin/ldconfig
```

Then, add the `docker_ng` service to the `hosts:`-line in `/etc/nsswitch.conf`. For example: `hosts: files docker_ng dns`

# Contributions

Contributions are welcome! Nothing to contribute, but you do appreciate this software? Please star :star: this repo.

# Development

```
cargo build && \
 patchelf --set-soname libnss_docker_ng.so.2 target/debug/libnss_docker_ng.so && \
 DESTDIR="/usr/local/lib/nss-docker-ng/" && \
 sudo --preserve-env=DESTDIR install -m 0755 -d "$DESTDIR" && \
 sudo --preserve-env=DESTDIR install -m 0644 target/debug/libnss_docker_ng.so "${DESTDIR}/libnss_docker_ng.so" && \
 echo "${DESTDIR}" | sudo tee /etc/ld.so.conf.d/nss-docker-ng.conf > /dev/null && \
 sudo /sbin/ldconfig

getent hosts not-existing-container.docker
# Failed to inspect container 'not-existing-container': error 404 Not Found - No such container: not-existing-container
```

# Comparison

There are other options to achieve this feature. I'm comparing them [HERE](COMPARISON.md).

# Useful links

* https://crates.io/crates/nss-docker-ng
* https://launchpad.net/~petski/+archive/ubuntu/nss-docker-ng
* https://docs.rs/docker-api/latest/docker_api/
* https://docs.docker.com/engine/api/

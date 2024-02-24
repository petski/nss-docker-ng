# Benchmarks

Command used: `hyperfine --export-markdown /dev/stdout 'getent hosts my-app.docker'`

| Command                                                         | Mean [ms]   | Min [ms] | Max [ms] | Relative |
|:----------------------------------------------------------------|------------:|---------:|---------:|---------:|
| [dex4er/nss-docker](https://github.com/dex4er/nss-docker)       | 7.8 ± 6.1   | 3.3      | 30.2     | 1.00     |
| [coredns](https://coredns.io/) [^1]                             | 0.5 ± 0.4   | 0.0      | 4.0      | 1.00     |
| [petski/nss-docker-ng](https://github.com/petski/nss-docker-ng) | 24.5 ± 16.0 | 8.1      | 67.6     | 1.00     |

[^1]: Inspired by https://theorangeone.net/posts/expose-docker-internal-dns/

    Run `export COREDNS_CONTAINER=$(docker run -d --name coredns -v /var/tmp/Corefile:/home/nonroot/Corefile:ro coredns/coredns:latest)` with `/var/tmp/Corefile` being:

    ```
    . {
        view docker {
            expr name() endsWith '.docker.'
        }
        rewrite name suffix .docker . answer auto
    
        errors
        cancel
    
        forward . 127.0.0.11
    }
    . {
        acl {
            block
        }
    }
    ```

    Add `--network=yournetwork` to the `docker run`-line if your my-app container is in a specific network.

    To determine the container address, run `COREDNS_IP=$(docker inspect -f '{{range.NetworkSettings.Networks}}{{.IPAddress}}{{end}}' $COREDNS_CONTAINER) && echo $COREDNS_IP`.

    Test with `dig @$COREDNS_IP my-app.docker. +short`

    Now set the IP `$COREDNS_IP` as your first DNS server, and leave the one(s) present as secondary ones.

    For a permanent solution, I would suggest giving the container a static IP and run it with `--restart=unless-stopped`.

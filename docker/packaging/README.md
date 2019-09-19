# Packaging

This directory contains docker images used for creating packages for different distribution.

##  How to use this docker files?

Start with:

```bash
$ docker build -f docker/packaging/Dockerfile.ubuntu-bionic -t nushell/package:ubuntu-bionic .
```

after building the image please run container:

```bash
$ docker run -td --rm --name nushell_package_ubuntu_bionic nushell/package:ubuntu-bionic
``` 

and copy deb package from inside:

```bash
$ docker cp nushell_package_ubuntu_bionic:/nu_0.2.0-1_amd64.deb .
```

or shell inside, and test install:

```bash
$ docker exec -it nushell_package_ubuntu_bionic bash
$ dpkg -i /nu_0.2.0-1_amd64.deb

(Reading database ... 25656 files and directories currently installed.)
Preparing to unpack /nu_0.2.0-1_amd64.deb ...
Unpacking nu (0.2.0-1) over (0.2.0-1) ...
Setting up nu (0.2.0-1) ...
```

When you are finished, exit and stop the container. It will be removed since we
used `--rm`.

```bash
$ docker stop nushell_package_ubuntu_bionic
```

## What should be done

* We should run sbuild command to create chroot and then install dpkg.
For two reasons. First: we want to use the same tools as Ubuntu package builders
to handle the cornercases. Second: we want to test dpkg requirements.
https://github.com/nushell/nushell/issues/681

* File debian/changelog file should be generated based on git history.
https://github.com/nushell/nushell/issues/682

* Building package and nu version should be parametrized.
https://github.com/nushell/nushell/issues/683
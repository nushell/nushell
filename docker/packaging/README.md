# Packaging

This directory contains docker images used for creating packages for different distribution.

##  How to use this docker files?

Start with:

`docker build -f docker/packaging/Dockerfile.ubuntu-bionic .`

after building the image please run container

`docker run -d --name nushell $(docker images -q -a | head -n+1)`
 
and copy deb package from inside:

`docker cp nushell:/nu_0.2.0-1_amd64.deb .`

## What should be done

* We should run sbuild command to create chroot and then install dpkg.
For two reasons. First: we want to use the same tools as Ubuntu package builders
to handle the cornercases. Second: we want to test dpkg requirements.
https://github.com/nushell/nushell/issues/681

* File debian/changelog file should be generated based on git history.
https://github.com/nushell/nushell/issues/682

* Building package and nu version should be parametrized.
https://github.com/nushell/nushell/issues/683
# Packaging

This directory contains docker images used for creating packages for different distribution.

##  How to use this docker files?

Start with:

`docker build -f docker/packaging/Dockerfile.ubuntu-bionic .`

after building the image please copy dpkg package from inside:

`docker cp $(docker ps -q -a | head -n1):/nu_0.2.0-1_amd64.deb .`

## What should be done

* We should run sbuild command to create chroot and then install dpkg.
For two reasons. First: we want to use the same tools as Ubuntu package builders
to handle the cornercases. Second: we want to test dpkg requirements.
* File debian/changelog file should be generated based on git history.
* Building package and nu version should be parametrized. 
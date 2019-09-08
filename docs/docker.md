# Docker Guide

| tag               | base image           | plugins | package manager | libs & bins                                                      | size        |
| ----------------- | -------------------- | ------- | --------------- | ---------------------------------------------------------------- | ----------- |
| `latest`,`debian` | `debian:latest`      | yes     | apt             | **a lot**, including _glibc_                                     | ~(48+62) MB |
| `slim`            | `debian:stable-slim` | yes     | apt             | all `nu:debian` image but exclude [this list][.slimify-excludes] | ~(26+62) MB |
| `alpine`          | `alpine:latest`      | yes     | apk             | all `nu:musl-busybox` image + libcrypto, libssl, libtls, libz    | ~(3+61) MB  |
| `musl-busybox`    | `busybox:musl`       | no      | —               | GNU utils + _musl_                                               | ~(1+16) MB  |
| `glibc-busybox`   | `busybox:glibc`      | no      | —               | GNU utils + _glibc_                                              | ~(3+17) MB  |

[.slimify-excludes]: https://github.com/debuerreotype/debuerreotype/blob/master/scripts/.slimify-excludes
[distroless/base]: https://github.com/GoogleContainerTools/distroless/blob/master/base/README.md

## Image Variants

### `nu:<version>`
This is the defacto image. If you are unsure about what your needs are, you probably want to use this one. It is designed to be used both as a throw away container (mount your source code and start the container to start your app), as well as the base to build other images off of.

<details><summary>example</summary>

Let say you create a plugin in Rust.
- create a Dockerfile in your root project
```dockerfile
FROM nu:0.2

COPY /target/debug/nu_plugin_cowsay /bin/
ENTRYPOINT ["nu"]
```
- build your project first then run it via docker
```console
cargo build
docker run -it .
```
</details>

### `nu:<version>-slim`
<!-- TODO: give a reason why you should use slim rather than alpine -->
This image does not contain the common packages contained in the default tag and only contains the minimal packages needed to run `nu`. Unless you are working in an environment where only the `nu` image will be deployed and you have space constraints, we highly recommend using the alpine image if you aim for small image size. Only use this image if you really need **both** `glibc` and small image size.

### `nu:<version>-alpine`
This image is based on the popular [Alpine Linux project](http://alpinelinux.org/), available in [the alpine official image][alpine]. Alpine Linux is much smaller than most distribution base images (~5MB), and thus leads to much slimmer images in general.

This variant is highly recommended when final image size being as small as possible is desired. The main caveat to note is that it does use `musl` libc instead of `glibc` and friends, so certain software might run into issues depending on the depth of their libc requirements. However, most software doesn't have an issue with this, so this variant is usually a very safe choice. See [this Hacker News comment thread](https://news.ycombinator.com/item?id=10782897) for more discussion of the issues that might arise and some pro/con comparisons of using Alpine-based images.

To minimize image size, it's uncommon for additional related tools (such as `git` or `bash`) to be included in Alpine-based images. Using this image as a base, add the things you need in your own Dockerfile (see the [alpine image description][alpine] for examples of how to install packages if you are unfamiliar).

### `nu:<version>-<libc-variant>-busybox`
This image is based on [Busybox](http://www.busybox.net/) which is a very good ingredient to craft space-efficient distributions. It combines tiny versions of many common UNIX utilities into a single small executable. It also provides replacements for most of the utilities you usually find in GNU fileutils, shellutils, etc. The utilities in BusyBox generally have fewer options than their full-featured GNU cousins; however, the options that are included provide the expected functionality and behave very much like their GNU counterparts. Basically, this image provides a fairly complete environment for any small or embedded system.

> Use this only if you need common utilities like `tar`, `awk`, and many more but don't want extra blob like nushell plugins and others.

<details><summary>example</summary>

```dockerfile
FROM nu:0.2-glibc-busybox

ADD https://github.com/user/repo/releases/download/latest/nu_plugin_cowsay.tar.gz /tmp/
RUN tar xzfv nu_plugin_cowsay.tar.gz -C /bin

ENTRYPOINT ["nu"]
```
</details>

[musl]: http://www.musl-libc.org/
[alpine]: https://hub.docker.com/_/alpine/
# Docker Guide

| tag                | base image           | plugins | package manager | libs & bins                                                      | size        |
| ------------------ | -------------------- | ------- | --------------- | ---------------------------------------------------------------- | ----------- |
| `latest`, `debian` | `debian:latest`      | yes     | apt             | **a lot**, including _glibc_                                     | ~(48+62) MB |
| `slim`             | `debian:stable-slim` | yes     | apt             | all `nu:debian` image but exclude [this list][.slimify-excludes] | ~(26+62) MB |
| `alpine`           | `alpine:latest`      | yes     | apk             | all `nu:musl-busybox` image + libcrypto, libssl, libtls, libz    | ~(3+61) MB  |
| `musl-busybox`     | `busybox:musl`       | no      | —               | GNU utils + _musl_                                               | ~(1+16) MB  |
| `glibc-busybox`    | `busybox:glibc`      | no      | —               | GNU utils + _glibc_                                              | ~(3+17) MB  |
| `musl-distroless`  | `distroless/static`  | no      | —               | see [here][distroless/base]                                      | ~(2+16) MB  |
| `glibc-distroless` | `distroless/cc`      | no      | —               | `distroless/static` with _glibc_                                 | ~(17+17) MB |
| `glibc`            | `scratch`            | no      | —               | **only `nu` binary-executable** which depend on glibc runtime    | ~17 MB      |
| `musl`             | `scratch`            | no      | —               | **only `nu` binary-executable** statically linked to musl        | ~16 MB      |

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
This image does not contain the common packages contained in the default tag and only contains the minimal packages needed to run `nu`. Unless you are working in an environment where only the `nu` image will be deployed and you have space constraints, it's highly recommended to use the alpine image if you aim for small image size. Only use this image if you really need **both** `glibc` and small image size.

### `nu:<version>-alpine`
This image is based on the popular [Alpine Linux project](https://alpinelinux.org/), available in [the alpine official image][alpine]. Alpine Linux is much smaller than most distribution base images (~5MB), and thus leads to much slimmer images in general.

This variant is highly recommended when final image size being as small as possible is desired. The main caveat to note is that it does use `musl` libc instead of `glibc` and friends, so certain software might run into issues depending on the depth of their libc requirements. However, most software doesn't have an issue with this, so this variant is usually a very safe choice. See [this Hacker News comment thread](https://news.ycombinator.com/item?id=10782897) for more discussion of the issues that might arise and some pro/con comparisons of using Alpine-based images.

To minimize image size, it's uncommon for additional related tools (such as `git` or `bash`) to be included in Alpine-based images. Using this image as a base, add the things you need in your own Dockerfile (see the [alpine image description][alpine] for examples of how to install packages if you are unfamiliar).

### `nu:<version>-<libc-variant>`
This image is based on [`scratch`](https://hub.docker.com/_/scratch) which doesn't create an extra layer. This variants can be handy in a project that uses multiple programming language as you need a lot of tools. By using this in [multi-stage build][], you can slim down the docker image that need to be pulled.

[multi-stage build]: https://docs.docker.com/develop/develop-images/multistage-build/

<details><summary>example</summary>

- using `glibc` variant
```dockerfile
FROM nu:0.2-glibc as shell
FROM node:slim

# Build your plugins

COPY --from=shell /bin/nu /bin/
# Something else
ENTRYPOINT ["nu"]
```

- using `musl` variant
```dockerfile
FROM nu:musl as shell
FROM go:alpine

# Build your plugins

COPY --from=shell /bin/nu /bin/
# Something else
ENTRYPOINT ["nu"]
```
</details>

### `nu:<version>-<libc-variant>-distroless`
This image is base on [Distroless](https://github.com/GoogleContainerTools/distroless) which usually to contain only your application and its runtime dependencies. This image do not contain package managers, shells or any other programs you would expect to find in a standard Linux distribution except for nushell itself. All distroless variant always contains:
- ca-certificates
- A /etc/passwd entry for a root user
- A /tmp directory
- tzdata

As for `glibc-distroless` variant, it **adds**:
- glibc
- libssl
- openssl

> Most likely you want to use this in CI/CD environment for plugins that can be statically compiled.

<details><summary>example</summary>

```dockerfile
FROM nu:musl-distroless

COPY target/x86_64-unknown-linux-musl/release/nu_plugin_* /bin/
ENTRYPOINT ["nu"]
```
</details>

### `nu:<version>-<libc-variant>-busybox`
This image is based on [Busybox](https://www.busybox.net/) which is a very good ingredient to craft space-efficient distributions. It combines tiny versions of many common UNIX utilities into a single small executable. It also provides replacements for most of the utilities you usually find in GNU fileutils, shellutils, etc. The utilities in BusyBox generally have fewer options than their full-featured GNU cousins; however, the options that are included provide the expected functionality and behave very much like their GNU counterparts. Basically, this image provides a fairly complete environment for any small or embedded system.

> Use this only if you need common utilities like `tar`, `awk`, and many more but don't want extra blob like nushell plugins and others.

<details><summary>example</summary>

```dockerfile
FROM nu:0.2-glibc-busybox

ADD https://github.com/user/repo/releases/download/latest/nu_plugin_cowsay.tar.gz /tmp/
RUN tar xzfv nu_plugin_cowsay.tar.gz -C /bin --strip=1 nu_plugin_cowsay

ENTRYPOINT ["nu"]
```
</details>

[musl]: https://www.musl-libc.org/
[alpine]: https://hub.docker.com/_/alpine/
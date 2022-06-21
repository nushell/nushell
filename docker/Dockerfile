# Git: git version 2.30.2
# /etc/os-release: Debian: Debian GNU/Linux 11 (bullseye)
# Kernel: Linux ec73d87a5aab 5.10.104-linuxkit #1 SMP Wed Mar 9 19:05:23 UTC 2022 x86_64 GNU/Linux
# Build cmd: docker build --no-cache . -t nushell-latest
# Other tags: nushell/debian-nu:latest, nushell
FROM debian:bullseye-slim

LABEL maintainer=nushell

RUN apt update \
    && apt upgrade -y \
    # Need ca-certificates to make `curl -s` work
    && apt install -y --no-install-recommends --no-install-suggests ca-certificates aria2 curl git unzip \
    # Make /bin/sh symlink to bash instead of dash:
    && echo "dash dash/sh boolean false" | debconf-set-selections \
    && DEBIAN_FRONTEND=noninteractive dpkg-reconfigure dash \
    && cd /lib; curl -s https://api.github.com/repos/nushell/nushell/releases/latest | grep browser_download_url | cut -d '"' -f 4 | grep x86_64-unknown-linux-gnu | aria2c -i - \
    && mkdir nu-latest && tar xvf nu-*.tar.gz --directory=nu-latest \
    && cp -aR nu-latest/* /usr/local/bin/ \
    # Setup default config file for nushell
    && mkdir -p /root/.config/nushell && cd /root/.config/nushell \
    && aria2c https://raw.githubusercontent.com/nushell/nushell/main/docs/sample_config/default_env.nu -o env.nu \
    && aria2c https://raw.githubusercontent.com/nushell/nushell/main/docs/sample_config/default_config.nu -o config.nu \
    # Do some cleanup work
    && cd /lib; rm -rf nu-* \
    && rm -rf /var/lib/apt/lists/* && apt autoremove -y \
    && echo '/usr/local/bin/nu' >> /etc/shells \
    # Add an nushell user and create home dir
    && useradd -m -s /usr/local/bin/nu nushell

CMD [ "nu" ]

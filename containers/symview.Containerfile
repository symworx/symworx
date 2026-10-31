# SymWorx interactive TUI (symview)
# Launches the keyboard-driven explorer on start.
#
#   podman run --rm -it -v "$PWD:/data:z" ghcr.io/symworx/symview:0.5.0
#   docker build -f containers/symview.Containerfile -t ghcr.io/symworx/symview:0.5.0 .
#   docker run --rm -it -v "$PWD:/data" ghcr.io/symworx/symview:0.5.0
#
# :z relabels the mount for SELinux. On Docker with SELinux, use -v "$PWD:/data:z".
FROM rust:1.86

WORKDIR /data

# Install the TUI binary.
# Release CI sets INSTALL_MODE=crates and SYMWORX_VERSION to the tag.
# Local default keeps the git fallback when that crates.io version is missing.
# Unpinned local build: --build-arg INSTALL_MODE=git
ARG SYMWORX_VERSION=0.5.0
ARG INSTALL_MODE=crates-or-git
RUN set -eux; \
    if [ "$INSTALL_MODE" = "crates" ]; then \
      cargo install symworx-tui --bin symview --version "${SYMWORX_VERSION}"; \
    elif [ "$INSTALL_MODE" = "git" ]; then \
      cargo install --git https://github.com/symworx/symworx --bin symview --branch worx; \
    else \
      cargo install symworx-tui --bin symview --version "${SYMWORX_VERSION}" || \
      cargo install --git https://github.com/symworx/symworx --bin symview --branch worx; \
    fi

# Mount host data at /data so users can drop CSV / Parquet / FIT files in.
# The TUI starts immediately; Ctrl+Q quits.
ENTRYPOINT ["symview"]

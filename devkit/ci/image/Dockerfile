# syntax=docker/dockerfile:1
#
# Toolchain image for the Linux C++ CI jobs (cpp.yml, codeql.yml). It holds
# everything that used to be installed by `bootstrap.sh` + two `apt-get` steps
# on every run: compilers, CMake, Ninja, Rust, cbindgen, the Qt xcb build
# libraries and the X11/Wayland/software-Vulkan test stack. Bootstrap is now a
# developer tool; the CI never runs it.
#
# What is NOT in here: vcpkg (a git submodule, pinned by the repository, see
# check-vcpkg.sh) and the vcpkg packages (binary cache, .github/actions/
# vcpkg-cache). LLVM, clang-tidy, LLDB, GDB, cppcheck and mdBook are not used by
# the C++ CI jobs (the presets use GCC) and stay out to keep the pull small.
#
# The image is published as ghcr.io/<owner>/coreverse-ci:<hash>. The hash is
# computed by .github/workflows/image.yml from THIS FILE and the minimum tool
# versions in devkit/scripts/setup/config/tool-versions.json, so changing either
# produces (and publishes) a new image; nothing else has to be edited.
# CMake, Ninja, Rust and cbindgen are "latest at build time" (like the Bootstrap
# they replace) and then frozen in the image: bump CI_IMAGE_REVISION to refresh.

FROM ubuntu:24.04

ARG CI_IMAGE_REVISION=1
ARG REPO_URL=https://github.com/Coreverse-Game-Engine/Coreverse-Engine.git
ARG CARGO_MIN
ARG CMAKE_MIN
ARG NINJA_MIN
ARG CBINDGEN_MIN
ARG GCC_MIN

LABEL org.opencontainers.image.source="${REPO_URL}" \
      org.opencontainers.image.description="Toolchain image for the Coreverse C++ CI jobs (devkit/ci/image/Dockerfile)"

ENV DEBIAN_FRONTEND=noninteractive \
    LANG=C.UTF-8 \
    RUSTUP_HOME=/opt/rustup \
    CARGO_HOME=/opt/cargo \
    PATH=/opt/venv/bin:/opt/cargo/bin:${PATH}

# Qt's "xcb" platform plugin is only built when the xcb/xkbcommon DEVELOPMENT
# packages exist at the moment vcpkg compiles qtbase (Qt silently drops the
# plugin otherwise), hence the *-dev list. The runtime libraries and the
# Xvfb/weston/lavapipe/validation-layer stack are what the GPU and editor smoke
# tests run on (see the comments on those steps in cpp.yml). The first block is
# what vcpkg and its autotools-based ports (libb2, a dependency of qtbase) need;
# bison/flex/gperf/nasm/yasm/libltdl-dev/libssl-dev are there because the
# GitHub runner image had them implicitly, and a bare Ubuntu image does not.
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates curl git jq zip unzip tar xz-utils zstd \
        build-essential pkg-config perl python3 python3-venv python3-pip \
        autoconf autoconf-archive automake libtool \
        bison flex gperf nasm yasm libltdl-dev libssl-dev \
        libfontconfig1-dev libfreetype-dev libx11-dev libx11-xcb-dev libxext-dev \
        libxfixes-dev libxi-dev libxrender-dev libgl1-mesa-dev libglu1-mesa-dev \
        libegl1-mesa-dev libxcb1-dev libxcb-cursor-dev libxcb-xinput-dev \
        libxcb-glx0-dev libxcb-icccm4-dev libxcb-image0-dev libxcb-keysyms1-dev \
        libxcb-randr0-dev libxcb-render0-dev libxcb-render-util0-dev \
        libxcb-shape0-dev libxcb-shm0-dev libxcb-sync-dev libxcb-util-dev \
        libxcb-xfixes0-dev libxcb-xinerama0-dev libxcb-xkb-dev libxkbcommon-dev \
        libxkbcommon-x11-dev \
        libwayland-dev weston xvfb xauth \
        mesa-vulkan-drivers libvulkan1 vulkan-tools vulkan-validationlayers \
        libxcb-cursor0 libxcb-icccm4 libxcb-image0 libxcb-keysyms1 libxcb-randr0 \
        libxcb-render-util0 libxcb-shape0 libxcb-xinerama0 libxcb-xkb1 \
        libxkbcommon-x11-0 \
    && rm -rf /var/lib/apt/lists/* \
    # The job container runs as root on a workspace owned by another user. \
    && git config --system --add safe.directory '*'

# CMake and Ninja from PyPI (maintained by Kitware / the Ninja project): no
# GitHub API rate limit at build time, unlike the release-tarball download.
RUN python3 -m venv /opt/venv \
    && /opt/venv/bin/pip install --no-cache-dir cmake ninja

# Rust (stable) with the components the C++ jobs do not need but the Rust
# tooling around them does, and cbindgen (the ffi header generator).
RUN curl -fsSL https://sh.rustup.rs \
        | sh -s -- -y --no-modify-path --profile minimal --default-toolchain stable \
            -c clippy -c rustfmt \
    && cargo install --locked cbindgen \
    && rm -rf /opt/cargo/registry /opt/cargo/git

# Fail the image build, not a CI run, if a tool is older than the project's
# stated minimum (tool-versions.json).
RUN set -eu; \
    ver_ge() { [ "$(printf '%s\n%s\n' "$2" "$1" | sort -V | sed -n 1p)" = "$2" ]; }; \
    check() { \
        if ver_ge "$2" "$3"; then echo "ok: $1 $2 (minimum $3)"; \
        else echo "ERROR: $1 $2 is older than the minimum $3 in tool-versions.json" >&2; exit 1; fi; \
    }; \
    check cargo    "$(cargo --version | awk '{print $2}')"          "$CARGO_MIN"; \
    check cmake    "$(cmake --version | awk 'NR==1{print $3}')"     "$CMAKE_MIN"; \
    check ninja    "$(ninja --version)"                              "$NINJA_MIN"; \
    check cbindgen "$(cbindgen --version | awk '{print $2}')"        "$CBINDGEN_MIN"; \
    check gcc      "$(gcc -dumpfullversion)"                         "$GCC_MIN"

# Printed by the CI jobs so a log shows exactly which tools ran.
RUN { \
        echo "image revision: ${CI_IMAGE_REVISION}"; \
        . /etc/os-release && echo "os: ${PRETTY_NAME}"; \
        echo "gcc: $(gcc --version | sed -n 1p)"; \
        echo "cmake: $(cmake --version | sed -n 1p)"; \
        echo "ninja: $(ninja --version)"; \
        echo "cargo: $(cargo --version)"; \
        echo "rustc: $(rustc --version)"; \
        echo "cbindgen: $(cbindgen --version)"; \
        echo "git: $(git --version)"; \
    } > /opt/ci-image-versions.txt

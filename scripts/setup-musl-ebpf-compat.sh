#!/usr/bin/env bash
set -euo pipefail

target="${1:-x86_64-unknown-linux-musl}"
env_file="${2:-}"

case "${target}" in
  x86_64-unknown-linux-musl)
    target_include="/usr/include/x86_64-linux-gnu"
    cc_env="CC_x86_64_unknown_linux_musl"
    linker_env="CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER"
    extra_link_libs=""
    ;;
  aarch64-unknown-linux-musl)
    target_include="/usr/include/aarch64-linux-gnu"
    cc_env="CC_aarch64_unknown_linux_musl"
    linker_env="CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER"
    extra_link_libs="-latomic -lgcc"
    ;;
  *)
    echo "unsupported musl target: ${target}" >&2
    exit 1
    ;;
esac

compat_root="${HERAKLES_MUSL_COMPAT_ROOT:-$(mktemp -d)}"
compat_prefix="${compat_root}/prefix"
mkdir -p "${compat_prefix}"

git -C "${compat_root}" clone --depth=1 --branch 1.4.1 https://github.com/ericonr/argp-standalone.git
(
  cd "${compat_root}/argp-standalone"
  autoreconf -fi
  CC=musl-gcc ./configure --prefix="${compat_prefix}" --disable-dependency-tracking
  make -j"$(nproc)"
  install -D argp.h "${compat_prefix}/include/argp.h"
  install -D libargp.a "${compat_prefix}/lib/libargp.a"
)

git -C "${compat_root}" clone --depth=1 --branch v1.2.3 https://github.com/void-linux/musl-obstack.git
(
  cd "${compat_root}/musl-obstack"
  ./bootstrap.sh
  CC=musl-gcc ./configure --prefix="${compat_prefix}" --disable-dependency-tracking
  make -j"$(nproc)"
  make install
)

git -C "${compat_root}" clone --depth=1 --branch v1.2.7 https://github.com/void-linux/musl-fts.git
(
  cd "${compat_root}/musl-fts"
  ./bootstrap.sh
  CC=musl-gcc ./configure --prefix="${compat_prefix}" --disable-dependency-tracking
  make -j"$(nproc)"
  make install
)

wrapper="${compat_root}/musl-compat-cc"
cat > "${wrapper}" <<EOF
#!/usr/bin/env bash
extra_link_libs="${extra_link_libs}"
if [ -n "\${extra_link_libs}" ]; then
  for arg in "\$@"; do
    if [ "\${arg}" = "-c" ]; then
      exec musl-gcc \
        -I${compat_prefix}/include \
        -idirafter /usr/include \
        -idirafter ${target_include} \
        -L${compat_prefix}/lib \
        "\$@"
    fi
  done
fi
exec musl-gcc \
  -I${compat_prefix}/include \
  -idirafter /usr/include \
  -idirafter ${target_include} \
  -L${compat_prefix}/lib \
  "\$@" \
  \${extra_link_libs}
EOF
chmod +x "${wrapper}"

if [ -n "${env_file}" ]; then
  cat > "${env_file}" <<EOF
BUILD_FEATURES=ebpf-vendored
${cc_env}=${wrapper}
${linker_env}=${wrapper}
LIBBPF_SYS_EXTRA_CFLAGS=-I${compat_prefix}/include -idirafter /usr/include -idirafter ${target_include}
LIBBPF_SYS_LIBRARY_PATH=${compat_prefix}/lib
HERAKLES_MUSL_COMPAT_ROOT=${compat_root}
HERAKLES_MUSL_COMPAT_PREFIX=${compat_prefix}
EOF
else
  cat <<EOF
BUILD_FEATURES=ebpf-vendored
${cc_env}=${wrapper}
${linker_env}=${wrapper}
LIBBPF_SYS_EXTRA_CFLAGS=-I${compat_prefix}/include -idirafter /usr/include -idirafter ${target_include}
LIBBPF_SYS_LIBRARY_PATH=${compat_prefix}/lib
HERAKLES_MUSL_COMPAT_ROOT=${compat_root}
HERAKLES_MUSL_COMPAT_PREFIX=${compat_prefix}
EOF
fi

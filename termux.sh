#!/usr/bin/env bash

set -euo pipefail

project_name="proxy-spider"
install_path="${HOME}/${project_name}"
download_path="${TMPDIR}/${project_name}.zip"

case $(getprop ro.product.cpu.abi) in
  "arm64-v8a")
    target="aarch64-linux-android"
    ;;
  "armeabi-v7a")
    if grep -qi 'neon' /proc/cpuinfo; then
      target="thumbv7neon-linux-androideabi"
    else
      target="armv7-linux-androideabi"
    fi
    ;;
  "armeabi")
    target="arm-linux-androideabi"
    ;;
  "x86")
    target="i686-linux-android"
    ;;
  "x86_64")
    target="x86_64-linux-android"
    ;;
  *)
    echo "Unsupported CPU ABI: ${abi}" >&2
    exit 1
    ;;
esac

curl -fLo "${download_path}" "https://nightly.link/threatcode/${project_name}/workflows/ci/main/${project_name}-${target}.zip"
mkdir -p "${install_path}"
unzip -qod "${install_path}" "${download_path}"
rm -f "${download_path}"
printf "%s installed successfully.\nRun 'cd %s && ./%s'.\n" "${project_name}" "${install_path}" "${project_name}"

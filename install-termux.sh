#!/bin/sh

set -eu

project_name="proxy-spider"
base_path="${HOME}"
install_path="${base_path}/${project_name}"
download_path="${TMPDIR}/${project_name}.zip"

[ -d "${install_path}" ] && rm -rf --interactive=once "${install_path}"
pkg upgrade --yes -o Dpkg::Options::='--force-confdef'
pkg install --yes python python-pip
curl -fsSLo "${download_path}" "https://github.com/threatcode/${project_name}/archive/refs/heads/main.zip"
unzip -d "${base_path}" "${download_path}"
rm -f "${download_path}"
mv "${install_path}-main" "${install_path}"
printf "%s installed successfully.\nRun 'cd %s && sh ./scripts/start-termux.sh'.\n" "${project_name}" "${install_path}"

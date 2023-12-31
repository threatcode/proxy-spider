#!/bin/sh
base_path=~
path="${base_path}/proxy-spider"
download_path="${PREFIX}/tmp/proxy-spider.zip"

pkg upgrade --yes -o Dpkg::Options::='--force-confdef' &&
pkg install --yes python python-pip &&
if [ -d "${path}" ]; then
    rm -rf --interactive=once "${path}"
fi &&
curl -fsSLo "${download_path}" 'https://github.com/threatcode/proxy-spider/archive/refs/heads/main.zip' &&
unzip -d "${base_path}" "${download_path}" &&
mv "${path}-main" "${path}" &&
python3 -m pip install -U --no-cache-dir --disable-pip-version-check setuptools wheel &&
python3 -m pip install -U --no-cache-dir --disable-pip-version-check -r "${path}/requirements-termux.txt" &&
printf "proxy-spider installed successfully.\nRun 'cd %s && sh start-termux.sh'.\n" "${path}"

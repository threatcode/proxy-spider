# 🚀 proxy-spider

[![CI](https://github.com/threatcode/proxy-spider/actions/workflows/ci.yml/badge.svg)](https://github.com/threatcode/proxy-spider/actions/workflows/ci.yml)

![TUI Demo](https://github.com/user-attachments/assets/0ac37021-d11c-4f68-b80d-bafdbaeb00bb)

**A lightning-fast, feature-rich proxy scraper and checker built in Rust.**

Collect, test, and organize HTTP/SOCKS4/SOCKS5 proxies from multiple sources with detailed metadata and intelligent filtering.

## ✨ Key Features

- **🔥 Blazing Performance** - Rust-powered async engine with configurable concurrency
- **🌍 Rich Metadata** - ASN, geolocation, and response time data via offline MaxMind databases
- **🎯 Smart Parsing** - Advanced regex engine extracts proxies from any format (`protocol://user:pass@host:port`)
- **🔐 Auth Support** - Handles username/password authentication seamlessly
- **📊 Interactive TUI** - Real-time progress monitoring with beautiful terminal interface
- **⚡ Flexible Output** - JSON (with metadata) and plain text formats
- **🎛️ Configurable** - Extensive options for sources, timeouts, and checking
- **📁 Local & Remote** - Supports both web URLs and local files as proxy sources
- **🐳 Docker Ready** - Containerized deployment with volume mounting

## 🔗 Related

Get pre-checked proxies from [threatcode/proxy-list](https://github.com/threatcode/proxy-list) - updated regularly using this tool.

## ⚠️ SAFETY WARNING ⚠️

This tool makes many network requests and can impact your IP-address reputation. Consider using a VPN for safer operation.

## 🚀 Quick Start

> All configuration options are documented in `config.toml` - edit it to customize sources, timeouts, and output preferences.

<details>
<summary>💻 Binary Installation</summary>

> **Note:** For Termux users, see the dedicated section below.

1. **Download** the appropriate binary from [nightly builds](https://nightly.link/threatcode/proxy-spider/workflows/ci/main?preview)
   - Not sure which one? Check the [platform support table](https://doc.rust-lang.org/beta/rustc/platform-support.html)
2. **Extract** the archive to a dedicated folder
3. **Configure** by editing `config.toml` to your needs
4. **Run** the executable

</details>

<details>
<summary>🐳 Docker Installation</summary>

> **Note:** Docker version uses a simplified log-based interface (no TUI).

1. **Install** [Docker Compose](https://docs.docker.com/compose/install/)
2. **Download** the docker archive from [nightly builds](https://nightly.link/threatcode/proxy-spider/workflows/ci/main?preview)
   - Look for artifacts named `proxy-spider-docker`
3. **Extract** to a folder and configure `config.toml`
4. **Build and run:**

   ```bash
   # Windows
   docker compose build
   docker compose up --no-log-prefix --remove-orphans

   # Linux/macOS
   docker compose build --build-arg UID=$(id -u) --build-arg GID=$(id -g)
   docker compose up --no-log-prefix --remove-orphans
   ```

</details>

<details>
<summary>📱 Termux Installation</summary>

> **Important:** Download Termux from [F-Droid](https://f-droid.org/en/packages/com.termux/), not Google Play ([why?](https://github.com/termux/termux-app#google-play-store-experimental-branch)).

1. **Auto-install** with one command:
   ```bash
   bash <(curl -fsSL 'https://raw.githubusercontent.com/threatcode/proxy-spider/main/termux.sh')
   ```
2. **Configure** using a text editor:
   ```bash
   nano ~/proxy-spider/config.toml
   ```
3. **Run the tool:**
   ```bash
   cd ~/proxy-spider && ./proxy-spider
   ```

## 🕹️ CLI Options

| Flag | Description |
|------|-------------|
| `-f, --file <FILE>` | Proxy file. |
| `-a, --address <ADDR>:<PORT>` | Run proxy server on specified address/port. |
| `-A, --auth <USER>:<PASS>` | Set authorization for proxy server. |
| `-d, --daemon` | Daemonize proxy server. |
| `-c, --check` | Perform proxy live check (connectivity, latency, and anonymity). |
| `--only-cc <CC>` | Filter proxies by country code (comma separated, e.g., US,CA). Applies to both checker and proxy server. |
| `--min-anonymity <LEVEL>` | Minimum anonymity level (transparent, anonymous, elite). |
| `--max-latency <LATENCY>` | Maximum latency allowed (e.g., 500ms, 1s, 0.5). Flexible units: ns, us, ms, s, m, h. |
| `--rank` | Sort active proxies by quality score (0-100). |
| `--top <N>` | Limit output to the top N proxies (requires ranking/profile). |
| `--profile <P>` | Use a predefined selector profile (`scraping`, `stealth`, `speed`). |
| `-t, --timeout <TIMEOUT>` | Max. time allowed for check/server (default: 30s). Supporting units: ns, us, ms, s, m, h. |
| `-r, --rotate <N>` | Rotate proxy IP for every N requests (default: 1). |
| `--rotate-on-error` | Rotate proxy IP and retry failed HTTP requests. |
| `--remove-on-error` | Remove proxy IP from proxy pool on failed HTTP requests. |
| `--max-errors <N>` | Max. errors allowed during rotation (default: 3). Use `-1` for indefinite. |
| `--max-redirs <N>` | Max. redirects allowed (default: 10). |
| `--max-retries <N>` | Max. retries for failed HTTP requests (default: 0). |
| `-m, --method <M>` | Rotation method (`sequent` or `random`). |
| `-s, --sync` | Sync mode: wait for the previous request to complete. |
| `-v, --verbose` | Dump HTTP request/responses (redacted) or show died proxies on check. |
| `-o, --output <FILE>` | Save output from proxy server or live check. |
| `--output-format <F>` | Custom format for checked proxies (e.g., `"{{protocol}}://{{host}}:{{port}}"`). |
| `-u, --update` | Update proxy-spider to the latest stable version. |
| `-w, --watch` | Watch proxy file, live-reload from changes. |
| `-V, --version` | Show current proxy-spider version. |

#### Available Template Variables:

| Variable | Description | Example |
|----------|-------------|---------|
| `{{proxy}}` | Full proxy URL | `http://1.2.3.4:8080` |
| `{{protocol}}` | Proxy protocol scheme | `http, socks5` |
| `{{host}}` | Proxy host/IP address | `1.2.3.4` |
| `{{port}}` | Proxy port | `8080` |
| `{{ip}}` | External IP address | `5.6.7.8` |
| `{{country}}` | Country code | `US, UK` |
| `{{city}}` | City name | `New York` |
| `{{org}}` | Organization/ISP | `Google Inc.` |
| `{{region}}` | Region/State | `NY, CA` |
| `{{timezone}}` | Timezone | `America/New_York` |
| `{{anonymity}}` | Anonymity level | `elite, anonymous` |
| `{{score}}` | Quality score (0-100) | `85, 98` |
| `{{loc}}` | Latitude,Longitude | `40.71, -74.00` |
| `{{hostname}}` | Hostname (reversed DNS) | `static-ip-1-2-3-4.isp.com` |
| `{{duration}}` | Response time | `245ms` |

### 📝 Operational Notes

- **Rotations** are counted for all requests, even if they fail.
- **Async Execution:** The proxy server runs asynchronously by default. Use `-s/--sync` for deterministic rotation if needed.
- **Daemon Mode:** Installs as a service (Linux/OSX/Windows). Activating daemon mode will force-uninstall existing instances before fresh installation.
- **Privacy:** Verbose mode redacts cookie values automatically and does not display request/reponse bodies.
- **Logging:** When using `-o/--output` for the proxy rotator, request/response headers are NOT written to the log file.
- **Error Handling:** `--max-errors` tracks total failures across all proxies, while `--max-retries` tracks retries for a single proxy. Rotation occurs after `max_retries` is reached, until `max_errors` limit.

## 📚 Documentation

- [Architecture Guide](docs/ARCHITECTURE.md) - System design and core components
- [API Reference](docs/API.md) - Public API overview
- [Metrics Guide](docs/metrics.md) - Observability and monitoring
- [Contribution Guidelines](docs/CONTRIBUTING.md) - How to help improve the project

## ❓ FAQ

**Q: Can I use this with SOCKS proxies?**  
A: Yes, it fully supports SOCKS4 and SOCKS5 protocols including authentication.

**Q: How do I change the checking URL?**  
A: Update the `check_url` field in your `config.toml`.

**Q: Is there a way to run this without the TUI?**  
A: Yes, the Docker version runs without the TUI by default.

## 🛠️ Troubleshooting

- **No proxies found:** Check your internet connection and ensure the source URLs in `config.toml` are accessible.
- **High failure rate:** Some proxies may be down or slow. Try increasing the `timeout` in your checking configuration.
- **Docker permission denied:** Ensure you are running with sufficient privileges or using the correct UID/GID arguments.

## 📄 License

[MIT](LICENSE)

 _This product includes GeoLite2 Data created by MaxMind, available from https://www.maxmind.com_

## 💖 Support

This project is maintained by [threatcode](https://github.com/threatcode). If you find it useful, please consider [sponsoring us on GitHub](https://github.com/sponsors/threatcode) to support ongoing development and maintenance.

## 🤝 Contributing

We welcome contributions! Please see [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md) for guidelines.

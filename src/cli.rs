use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "proxy-spider", version, about = "Proxy server and checker", long_about = None)]
pub struct Cli {

impl Cli {
    pub fn apply_to_config(&self, config: &mut crate::config::Config) {
        // Server Auth
        if let Some(ref auth) = self.auth {
            config.server.auth = Some(auth.clone());
        }
        // Server Address
        if let Some(ref addr) = self.address {
            if let Ok(sock_addr) = addr.parse() {
                config.server.bind_addr = sock_addr;
            }
        }
        // Rotation Method
        config.server.rotation_method = self.method.clone();
        if let Some(rotate) = self.rotate {
            config.server.rotate_after_requests = rotate;
        }
        config.server.rotate_on_error = self.rotate_on_error;
        config.server.remove_on_error = self.remove_on_error;
        config.server.max_errors = self.max_errors;
        config.server.max_redirs = self.max_redirs;
        config.server.max_retries = self.max_retries;
        // Country Filter
        if let Some(ref cc) = self.only_cc {
            config.server.country_filter = Some(cc.split(',').map(|s| s.trim().to_uppercase()).collect());
        }
        config.server.sync = self.sync;
        config.server.verbose = self.verbose;
        config.server.output = self.output.clone();
    }
}

    /// Proxy file to load.
    #[arg(short, long)]
    pub file: Option<PathBuf>,

    /// Run proxy server on address (HOST:PORT).
    #[arg(short = 'a', long)]
    pub address: Option<String>,

    /// Set authorization for proxy server (USER:PASS).
    #[arg(short = 'A', long)]
    pub auth: Option<String>,

    /// Daemonize proxy server (Not implemented).
    #[arg(short, long)]
    pub daemon: bool,

    /// To perform proxy live check (Default behavior).
    #[arg(short, long)]
    pub check: bool,

    /// Only show specific country code (comma separated, e.g. US,CN).
    #[arg(long, value_name = "CC")]
    pub only_cc: Option<String>,

    /// Max. time allowed for proxy server/check (seconds).
    #[arg(short, long)]
    pub timeout: Option<f64>,

    /// Rotate proxy IP for every AFTER request (default: 1).
    #[arg(short, long)]
    pub rotate: Option<usize>,

    /// Rotate proxy IP and retry failed HTTP requests.
    #[arg(long)]
    pub rotate_on_error: bool,

    /// Remove proxy IP from proxy pool on failed HTTP requests.
    #[arg(long)]
    pub remove_on_error: bool,

    /// Max. errors allowed during rotation (default: 3).
    #[arg(long)]
    pub max_errors: Option<isize>,

    /// Max. redirects allowed.
    #[arg(long)]
    pub max_redirs: Option<usize>,

    /// Max. retries for failed HTTP requests.
    #[arg(long)]
    pub max_retries: Option<usize>,

    /// Rotation method (sequent/random).
    #[arg(short, long, default_value = "random")]
    pub method: String,

    /// Sync will wait for the previous request to complete (Concurrency = 1).
    #[arg(short, long)]
    pub sync: bool,

    /// Dump HTTP request/responses or show died proxy on check.
    #[arg(short, long)]
    pub verbose: bool,

    /// Save output from proxy server or live check.
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Update Proxy spider to the latest stable version (Not implemented).
    #[arg(short, long)]
    pub update: bool,

    /// Watch proxy file, live-reload from changes (Not implemented).
    #[arg(short, long)]
    pub watch: bool,
}

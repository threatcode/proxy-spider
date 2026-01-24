use clap::Parser;
use std::path::PathBuf;

use crate::config::Profile;
use crate::proxy::AnonymityLevel;

#[derive(Parser, Debug)]
#[command(name = "proxy-spider", version, about = "Proxy server and checker", long_about = None)]
pub struct Cli {
    /// Proxy file.
    #[arg(short, long)]
    pub file: Option<PathBuf>,

    /// Run proxy server.
    #[arg(short = 'a', long)]
    pub address: Option<String>,

    /// Set authorization for proxy server.
    #[arg(short = 'A', long)]
    pub auth: Option<String>,

    /// Daemonize proxy server.
    #[arg(short, long)]
    pub daemon: bool,

    /// To perform proxy live check.
    #[arg(short, long)]
    pub check: bool,

    /// Only show specific country code (comma separated).
    #[arg(long, value_name = "CC")]
    pub only_cc: Option<String>,

    /// Max. time allowed for proxy server/check (default: 30s).
    #[arg(short, long)]
    pub timeout: Option<String>,

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
    /// Use this with --rotate-on-error.
    /// If value is less than 0 (e.g., -1), rotation will continue indefinitely.
    #[arg(long, value_name = "N")]
    pub max_errors: Option<isize>,

    /// Max. redirects allowed (default: 10).
    #[arg(long, value_name = "N")]
    pub max_redirs: Option<usize>,

    /// Max. retries for failed HTTP requests (default: 0).
    #[arg(long, value_name = "N")]
    pub max_retries: Option<usize>,

    /// Rotation method (sequent/random) (default: sequent).
    #[arg(short, long, default_value = "sequent")]
    pub method: String,

    /// Sync will wait for the previous request to complete.
    #[arg(short, long)]
    pub sync: bool,

    /// Dump HTTP request/responses or show died proxy on check.
    #[arg(short, long)]
    pub verbose: bool,

    /// Save output from proxy server or live check.
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Custom output format for checked proxies (e.g., "{{protocol}}://{{host}}:{{port}}").
    #[arg(long, value_name = "FORMAT")]
    pub output_format: Option<String>,

    /// Filter by minimum anonymity level (transparent, anonymous, elite).
    #[arg(long, value_enum)]
    pub min_anonymity: Option<AnonymityLevel>,

    /// Filter by maximum latency (e.g., "500ms", "0.5s", "500").
    #[arg(long, value_name = "LATENCY")]
    pub max_latency: Option<String>,

    /// Sort proxies by quality score (0-100).
    #[arg(long)]
    pub rank: bool,

    /// Limit output to top N proxies (requires --rank or --profile).
    #[arg(long, value_name = "N")]
    pub top: Option<usize>,

    /// Use a predefined selector profile (scraping, stealth, speed).
    #[arg(long, value_enum)]
    pub profile: Option<Profile>,

    /// Update proxy-spider to the latest stable version.
    #[arg(short, long)]
    pub update: bool,

    /// Watch proxy file, live-reload from changes.
    #[arg(short, long)]
    pub watch: bool,
}

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
        // Timeout
        if let Some(ref timeout_str) = self.timeout {
            if let Ok(duration) = humantime::parse_duration(timeout_str) {
                config.checking.timeout = duration;
                config.scraping.timeout = duration;
                config.server.timeout = duration;
            } else {
                // Try parsing strictly as seconds (f64) if humantime fails (fallback)
                if let Ok(secs) = timeout_str.parse::<f64>() {
                    let duration = std::time::Duration::from_secs_f64(secs);
                    config.checking.timeout = duration;
                    config.scraping.timeout = duration;
                    config.server.timeout = duration;
                } else {
                    tracing::warn!("Failed to parse timeout: {}", timeout_str);
                }
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
        if let Some(ref cc) = self.only_cc {
            let cc_vec: Vec<String> =
                cc.split(',').map(|s| s.trim().to_uppercase()).collect();
            config.server.country_filter = Some(cc_vec.clone());
            config.output.filters.only_cc = Some(cc_vec);
        }
        config.server.sync = self.sync;
        config.server.verbose = self.verbose;
        if let Some(ref output) = self.output {
            config.server.output = Some(output.clone());
        }
        if let Some(ref format) = self.output_format {
            config.output.txt.format = Some(format.clone());
        }

        // Ranking and Profiles
        config.output.rank = self.rank;
        config.output.top = self.top;
        config.output.profile = self.profile;

        if let Some(profile) = self.profile {
            profile.apply(&mut config.output.filters);
        }

        // Output Filters
        if let Some(min_anon) = self.min_anonymity {
            config.output.filters.min_anonymity = Some(min_anon);
        }
        if let Some(ref latency_str) = self.max_latency {
            if let Ok(duration) = humantime::parse_duration(latency_str) {
                config.output.filters.max_latency = Some(duration);
            } else if let Ok(secs) = latency_str.parse::<f64>() {
                config.output.filters.max_latency =
                    Some(std::time::Duration::from_secs_f64(secs));
            } else if let Ok(ms) = latency_str.parse::<u64>() {
                config.output.filters.max_latency =
                    Some(std::time::Duration::from_millis(ms));
            } else {
                tracing::warn!("Failed to parse max-latency: {}", latency_str);
            }
        }
    }
}

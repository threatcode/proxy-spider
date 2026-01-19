#![deny(
    warnings,
    deprecated_safe,
    future_incompatible,
    keyword_idents,
    let_underscore,
    nonstandard_style,
    refining_impl_trait,
    rust_2018_compatibility,
    rust_2018_idioms,
    rust_2021_compatibility,
    rust_2024_compatibility,
    unused,
    clippy::all,
    clippy::pedantic,
    clippy::restriction,
    clippy::nursery,
    clippy::cargo
)]
#![allow(
    clippy::absolute_paths,
    clippy::allow_attributes_without_reason,
    clippy::arbitrary_source_item_ordering,
    clippy::as_conversions,
    clippy::blanket_clippy_restriction_lints,
    clippy::cast_precision_loss,
    clippy::cognitive_complexity,
    clippy::else_if_without_else,
    clippy::float_arithmetic,
    clippy::implicit_return,
    clippy::integer_division_remainder_used,
    clippy::iter_over_hash_type,
    clippy::min_ident_chars,
    clippy::missing_docs_in_private_items,
    clippy::mod_module_files,
    clippy::multiple_crate_versions,
    clippy::pattern_type_mismatch,
    clippy::question_mark_used,
    clippy::separated_literal_suffix,
    clippy::shadow_reuse,
    clippy::shadow_unrelated,
    clippy::single_call_fn,
    clippy::single_char_lifetime_names,
    clippy::std_instead_of_alloc,
    clippy::std_instead_of_core,
    clippy::too_many_lines,
    clippy::unwrap_used
)]

#[cfg(all(feature = "dhat", feature = "mimalloc"))]
compile_error!(
    "Features 'dhat-heap' and 'mimalloc' are mutually exclusive. Enable only \
     one."
);

use color_eyre::eyre::WrapErr as _;
use proxy_spider::{config, create_logging_filter};

#[cfg(feature = "dhat")]
#[global_allocator]
static GLOBAL: dhat::Alloc = dhat::Alloc;

#[cfg(all(
    feature = "mimalloc",
    any(target_arch = "aarch64", target_arch = "x86_64"),
    any(target_os = "linux", target_os = "macos", target_os = "windows"),
))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[tokio::main]
#[expect(clippy::unwrap_in_result)]
async fn main() -> color_eyre::Result<()> {
    #[cfg(feature = "dhat")]
    let _profiler = dhat::Profiler::new_heap();

    color_eyre::install().wrap_err("failed to install color_eyre hooks")?;

    proxy_spider::metrics::init(None).wrap_err("failed to initialize metrics")?;

    let cli = proxy_spider::cli::Cli::parse();
    let mut config = config::load_config().await?;
    cli.apply_to_config(Arc::make_mut(&mut config));
    let logging_filter = create_logging_filter(&config);

    #[cfg(feature = "tui")]
    {
        proxy_spider::run_with_tui(config, logging_filter).await
    }
    #[cfg(not(feature = "tui"))]
    {
        proxy_spider::run_without_tui(config, logging_filter).await
    }
}

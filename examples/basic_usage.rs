//! Basic usage example for `proxy-spider`.
//!
//! This example demonstrates how to load the default configuration
//! and run the main scraping and checking task programmatically.

use std::sync::Arc;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize logging
    tracing_subscriber::fmt::init();

    // 2. Load default configuration (requires config.toml in current directory)
    let config = match proxy_spider::config::load_config().await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load configuration: {e}");
            eprintln!("Ensure a valid config.toml exists in the current directory.");
            return Ok(());
        }
    };

    // 3. Create a cancellation token for graceful shutdown
    let token = CancellationToken::new();

    // 4. Run the main task
    println!("Starting proxy scraper and checker...");
    
    // In a real application, you might want to run this in a separate task
    // and wait for signals to cancel the token.
    proxy_spider::main_task(
        config,
        token,
        #[cfg(feature = "tui")]
        tokio::sync::mpsc::unbounded_channel().0, // Dummy sender if tui enabled
    ).await?;

    println!("Process completed successfully.");
    Ok(())
}

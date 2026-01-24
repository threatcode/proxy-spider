use color_eyre::eyre::WrapErr as _;
use service_manager::{
    ServiceInstallCtx, ServiceLabel, ServiceManager, ServiceStartCtx,
    ServiceStopCtx, ServiceUninstallCtx,
};
use std::ffi::OsString;

pub const SERVICE_NAME: &str = "proxy-spider";

pub fn setup_daemon(args: Vec<String>) -> crate::Result<()> {
    let label: ServiceLabel = SERVICE_NAME.parse().unwrap();
    let manager = <dyn ServiceManager>::native().map_err(|e| {
        color_eyre::eyre::eyre!("Failed to get native service manager: {}", e)
    })?;

    // 1. Forcibly stop and uninstall existing service
    println!("Stopping existing service if running...");
    let _unused = manager.stop(ServiceStopCtx { label: label.clone() });

    println!("Uninstalling existing service...");
    let _unused =
        manager.uninstall(ServiceUninstallCtx { label: label.clone() });

    // 2. Install new service
    let current_exe = std::env::current_exe()
        .wrap_err("Failed to get current executable path")?;

    // We want the service to run with the provided args, but without the program name (args[0])
    // and without the -d/--daemon flags.
    let service_args: Vec<OsString> = args
        .into_iter()
        .skip(1)
        .filter(|a| a != "-d" && a != "--daemon")
        .map(OsString::from)
        .collect();

    println!(
        "Installing service: {} {:?}",
        current_exe.display(),
        service_args
    );

    manager
        .install(ServiceInstallCtx {
            label: label.clone(),
            program: current_exe,
            args: service_args,
            contents: None,
            username: None,
            working_directory: None,
            environment: None,
            restart_policy: service_manager::RestartPolicy::Always {
                delay_secs: None,
            },
            autostart: true,
        })
        .wrap_err("Failed to install service")?;

    // 3. Start service
    println!("Starting service...");
    manager
        .start(ServiceStartCtx { label: label.clone() })
        .wrap_err("Failed to start service")?;

    println!("Service successfully installed and started.");
    Ok(())
}

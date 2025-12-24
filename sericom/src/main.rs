//! Sericom is a CLI tool for communicating with devices over a serial connection.
//!
//! Currently, Sericom runs similarily to another CLI tool called 'screen'. In the future,
//! Sericom plans to allow for users to create config files for customizing appearances
//! and defaults. Sericom also plans to allow the writing of custom scripts (similar to
//! expect scripts) that can be parsed and executed by Sericom. The intention of these
//! scripts is to be able to automate tasks that take place over a serial connection i.e.
//! configuration, resetting, getting statistics, etc.

use miette::{Context as _, IntoDiagnostic};
use sericom_core::configs::{get_config, initialize_config};
use std::path::Path;
use tracing_subscriber::filter::FilterExt;

mod repl;
use repl::*;

#[tokio::main]
async fn main() -> miette::Result<()> {
    initialize_config(None)?;
    let _trace_guard: Option<tracing_appender::non_blocking::WorkerGuard> = {
        let config = get_config().unwrap();
        let dbg_dir = config.defaults.debug_dir.as_path();
        init_tracing(dbg_dir)?
    };

    run_repl().await
}

fn init_tracing(
    dbg_dir: &Path,
) -> miette::Result<Option<tracing_appender::non_blocking::WorkerGuard>> {
    use sericom_core::compat_port_path;
    use tracing::level_filters::LevelFilter;
    use tracing_subscriber::EnvFilter;
    use tracing_subscriber::layer::{Layer, SubscriberExt};
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::{filter, fmt};

    let path = compat_port_path!(trace, dbg_dir);
    let file = std::fs::File::options()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .into_diagnostic()
        .wrap_err_with(|| format!("Failed to create '{}'", path.display()))?;

    let (non_blocking, guard) = tracing_appender::non_blocking(file);

    let targets_filter = filter::Targets::new()
        .with_target("sericom_core", tracing::Level::TRACE)
        .with_target("sericom", tracing::Level::TRACE)
        .with_default(tracing::Level::ERROR);
    let env_filter = EnvFilter::builder()
        .with_default_directive(
            //     #[cfg(debug_assertions)]
            //     LevelFilter::TRACE.into(),
            //     #[cfg(not(debug_assertions))]
            LevelFilter::INFO.into(),
        )
        .with_env_var("SERI_LOG")
        .from_env_lossy();

    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_writer(non_blocking)
                .with_line_number(false)
                .with_target(true),
        )
        .with(targets_filter.and_then(env_filter))
        .init();

    Ok(Some(guard))
}

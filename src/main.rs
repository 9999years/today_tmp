#![allow(unused_imports)]

use std::path::PathBuf;

use chrono::{DateTime, Local};
use color_eyre::eyre::{self, WrapErr};
use structopt::StructOpt;
use tracing::{event, info, instrument, span, warn, Level};

use today_tmp::*;

#[instrument]
fn main() -> eyre::Result<()> {
    let args = Opt::from_args();
    install_tracing(&args.tracing_filter);
    color_eyre::install()?;

    create_repo_path(&args.repo_path)?;
    git_init(&args.repo_path)?;

    let today_dir = args
        .repo_path
        .join(format!("{}", Local::now().format(DATE_FMT)));

    if !today_dir.exists() {
        info!("Creating {}", &today_dir.display());
        std::fs::create_dir_all(&today_dir)
            .wrap_err_with(|| format!("Failed to create {:?}", &today_dir))?;
    }

    ensure_symlink(&args.today_path, &today_dir)?;

    // git commit
    // remove empty dirs
    // make prev link

    Ok(())
}

fn install_tracing(filter_directives: &str) {
    use tracing_error::ErrorLayer;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{
        fmt::{self, format::FmtSpan, time::ChronoLocal},
        EnvFilter,
    };

    let fmt_layer = fmt::layer()
        .with_target(false)
        .with_span_events(FmtSpan::ACTIVE);
    let filter_layer = EnvFilter::try_new(filter_directives)
        .or_else(|_| EnvFilter::try_from_default_env())
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(ErrorLayer::default())
        .init();
}

/// Manage a daily scratch directory.
#[derive(Debug, StructOpt)]
struct Opt {
    /// Tracing filter.
    ///
    /// Can be any of "error", "warn", "info", "debug", or
    /// "trace". Supports more granular filtering, as well; see documentation for
    /// `tracing_subscriber::EnvFilter`.
    // [EnvFilter]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/struct.EnvFilter.html
    #[structopt(long, default_value = "info")]
    tracing_filter: String,

    #[structopt(long, parse(from_os_str))]
    repo_path: PathBuf,

    #[structopt(long, parse(from_os_str))]
    today_path: PathBuf,
}

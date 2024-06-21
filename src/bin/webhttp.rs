use time::UtcOffset;
use tracing_subscriber::{fmt, prelude::*};
use webhttp;
const LOG_TOKEN: &str = "webhttp";

fn get_default_log_path(prjname: String) -> anyhow::Result<std::path::PathBuf> {
    let base_dir = directories::BaseDirs::new().unwrap();
    let mut log_path = base_dir.data_dir().to_path_buf();
    log_path.push(prjname);

    if !log_path.exists() {
        std::fs::create_dir_all(&log_path)?;
    }
    anyhow::Ok(log_path)
}

fn main() -> anyhow::Result<()> {
    let log_path = get_default_log_path(LOG_TOKEN.into())?;
    let file_appender = tracing_appender::rolling::daily(log_path.clone(), LOG_TOKEN);
    let (filelog, _guard) = tracing_appender::non_blocking(file_appender);
    let (stdoutlog, _guard) = tracing_appender::non_blocking(std::io::stdout());
    let local_time = tracing_subscriber::fmt::time::OffsetTime::new(
        UtcOffset::from_hms(8, 0, 0).unwrap(),
        time::format_description::well_known::Rfc3339,
    );

    let subscriber = tracing_subscriber::registry()
        .with(
            fmt::Layer::new()
                .with_writer(stdoutlog.with_max_level(tracing::Level::INFO))
                .with_timer(local_time.clone())
                .with_ansi(true)
                .with_target(true)
                .with_file(false)
                .with_line_number(true)
                .with_thread_ids(true)
                .with_thread_names(false), // .pretty(),
        )
        .with(
            fmt::Layer::new()
                .with_writer(filelog.with_max_level(tracing::Level::INFO))
                .with_timer(local_time.clone())
                .with_ansi(false)
                .with_target(false)
                .with_file(true)
                .with_line_number(true)
                .with_thread_ids(false)
                .with_thread_names(false),
        );
    tracing::subscriber::set_global_default(subscriber).unwrap();
    webhttp::webhttp::server_main()?;
    Ok(())
}

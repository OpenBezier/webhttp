use time::{macros::format_description, UtcOffset};
use tracing_subscriber::{fmt, prelude::*};

// #[actix_web::main]
fn main() -> anyhow::Result<()> {
    let local_time = tracing_subscriber::fmt::time::OffsetTime::new(
        UtcOffset::from_hms(8, 0, 0).unwrap(),
        format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:6]"),
    );
    let (stdoutlog, _guard) = tracing_appender::non_blocking(std::io::stdout());
    let subscriber = tracing_subscriber::registry().with(
        fmt::Layer::new()
            .with_writer(stdoutlog.with_max_level(tracing::Level::INFO))
            .with_timer(local_time.clone())
            // .without_time()
            .with_ansi(false)
            .with_target(true)
            .with_file(true)
            .with_line_number(true)
            .with_thread_ids(true)
            .with_level(true), // .pretty(),
    );
    tracing::subscriber::set_global_default(subscriber).unwrap();

    #[cfg(feature = "seaorm")]
    httpserver::wsserver::start_wsserver("ws_server".into());
    anyhow::Ok(())
}

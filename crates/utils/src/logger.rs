use time::macros::format_description;
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::{
    Registry,
    filter::EnvFilter,
    fmt::{
        self, FmtContext, FormatEvent, FormatFields, format::Writer, time::FormatTime,
        time::LocalTime,
    },
    layer::SubscriberExt,
    prelude::*,
    registry::LookupSpan,
};

pub fn attach_logger() {
    let timer = LocalTime::new(format_description!("[hour repr:24]:[minute]:[second]"));

    let default_level = if cfg!(debug_assertions) {
        "info"
    } else {
        "error"
    };

    let mut env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level));

    let directives = [
        "wgpu=error",
        "wgpu_hal=error",
        "naga=warn",
        "bevy_render=info",
        "bevy_app=info",
        "symphonia=warn",
    ];
    for directive in directives {
        env_filter = env_filter.add_directive(directive.parse().unwrap());
    }

    let format = VantablockFormatter { timer };

    let fmt_layer = fmt::layer().event_format(format);

    #[cfg(feature = "tracy")]
    let fmt_layer = fmt_layer.with_ansi(false);

    let subscriber = Registry::default().with(env_filter).with(fmt_layer);

    #[cfg(feature = "tracy")]
    let subscriber = subscriber.with(tracing_tracy::TracyLayer::default());

    subscriber.init();
}

/// A custom formatter that prints the time, a colored prefix for crate separation,
/// and a short level indicator.
pub struct VantablockFormatter<T> {
    timer: T,
}

impl<T: FormatTime, S, N> FormatEvent<S, N> for VantablockFormatter<T>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> std::fmt::Result {
        write!(writer, "\x1b[90m")?;
        self.timer.format_time(&mut writer)?;
        write!(writer, "\x1b[0m ")?;

        let target = event.metadata().target();

        #[allow(unused_assignments)]
        let mut prefix = "";

        if target.contains("server") {
            prefix = "\x1b[35m[SERVER]\x1b[0m "; // magenta
        } else if target.contains("client") {
            prefix = "\x1b[36m[CLIENT]\x1b[0m "; // cyan
        } else if target.contains("shared") {
            prefix = "\x1b[33m[SHARED]\x1b[0m "; // yellow
        } else {
            prefix = "\x1b[37m[EXTERN]\x1b[0m "; // gray
        }

        if !prefix.is_empty() {
            write!(writer, "{}", prefix)?;
        }

        let level = *event.metadata().level();
        let (level_char, level_color) = match level {
            Level::TRACE => ("T", "\x1b[35m"), // magenta
            Level::DEBUG => ("D", "\x1b[34m"), // blue
            Level::INFO => ("I", "\x1b[32m"),  // green
            Level::WARN => ("W", "\x1b[33m"),  // yellow
            Level::ERROR => ("E", "\x1b[31m"), // red
        };
        write!(writer, "{}{}\x1b[0m ", level_color, level_char)?;

        // final message
        ctx.format_fields(writer.by_ref(), event)?;
        writeln!(writer)
    }
}

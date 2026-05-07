use std::sync::atomic::{AtomicU8, Ordering};
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

// 0 = Unknown/Shared, 1 = Client, 2 = Server
static RUNTIME_CONTEXT: AtomicU8 = AtomicU8::new(0);

pub fn set_runtime_context_client() {
    RUNTIME_CONTEXT.store(1, Ordering::Relaxed);
}
pub fn set_runtime_context_server() {
    RUNTIME_CONTEXT.store(2, Ordering::Relaxed);
}

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

    let _ = subscriber.try_init();
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

        if target.contains("server") {
            write!(writer, "\x1b[35m[SERVER]\x1b[0m ")?;
        } else if target.contains("client") {
            write!(writer, "\x1b[36m[CLIENT]\x1b[0m ")?;
        } else if target.contains("shared") {
            // give shared crate context brackets so we know if caller was server or client
            let bracket_color = match RUNTIME_CONTEXT.load(Ordering::Relaxed) {
                1 => "\x1b[36m", // cyan (running on Client)
                2 => "\x1b[35m", // magenta (running on Server)
                _ => "\x1b[33m", // yellow (unknown)
            };
            write!(writer, "{}[", bracket_color)?;
            write!(writer, "\x1b[33mSHARED")?;
            write!(writer, "{}]\x1b[0m ", bracket_color)?;
        } else {
            write!(writer, "\x1b[37m[EXTERN]\x1b[0m ")?;
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

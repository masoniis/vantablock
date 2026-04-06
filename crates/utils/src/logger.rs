use time::macros::format_description;
use tracing_subscriber::{
    Registry, filter::EnvFilter, fmt, fmt::time::LocalTime, layer::SubscriberExt, prelude::*,
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

    let fmt_layer = fmt::layer()
        .with_target(false)
        .with_line_number(false)
        .with_thread_names(false)
        .with_file(false)
        .with_timer(timer)
        .compact();

    #[cfg(feature = "tracy")]
    let fmt_layer = fmt_layer.with_ansi(false);

    let subscriber = Registry::default().with(env_filter).with(fmt_layer);

    #[cfg(feature = "tracy")]
    let subscriber = subscriber.with(tracing_tracy::TracyLayer::default());

    subscriber.init();
}

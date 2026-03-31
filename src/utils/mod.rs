pub mod logger;
pub mod paths;
pub mod tracy_alloc;

pub use logger::attach_logger;
pub use paths::{get_resource_path, get_user_config_path};

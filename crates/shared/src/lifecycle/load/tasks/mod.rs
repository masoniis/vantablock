//! This module contains generic loading tasks that do not belong to any other specific module.
//!
//! These tasks typically represent miscellaneous work or utilities (like the `fake_work_system`)
//! that are orchestrated by the higher-level loading framework.

pub mod fake_work_system;

pub use fake_work_system::*;

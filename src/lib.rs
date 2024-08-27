//! # Kovi
//!
//! A OneBot V11 bot framework developed using Rust.
//! The project is in beta status.
//!
//! More documentation can be found at [Github-Kovi](https://github.com/Threkork/Kovi) Or [Kovi-doc](https://threkork.github.io/kovi-doc/)
//!
//! 中文文档或更多文档请查看[Github-Kovi](https://github.com/Threkork/Kovi) 和 [Kovi-doc](https://threkork.github.io/kovi-doc/)

/// Everything about bots is inside
pub mod bot;
pub mod error;

/// 提供一些方便的插件开发函数
#[cfg(feature = "utils")]
pub mod utils;

#[cfg(feature = "logger")]
mod logger;

pub use bot::message::Message;
pub use bot::plugin_builder::PluginBuilder;
pub use kovi_macros::plugin;

pub use log;
pub use serde;
pub use serde_json;
pub use tokio;
#[cfg(feature = "utils")]
pub use toml;

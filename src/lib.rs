//! # Kovi
//!
//! A OneBot V11 bot framework developed using Rust.
//! The project is in beta status.
//!
//! More documentation can be found at [Github-Kovi](https://github.com/Threkork/Kovi) Or [Kovi-doc](https://www.threkork.me/kovi-doc)


mod log;

pub use bot::message::Message;
pub use bot::plugin_builder::PluginBuilder;
pub use kovi_macros::plugin;

/// Everything about bots is inside
pub mod bot;
pub mod error;

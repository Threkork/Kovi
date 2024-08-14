//！ # Kovi
//！
//！ A **fast and lightweight** OneBot V11 bot framework developed using Rust.
//！ The project is in beta status, currently featuring **message listening** and **basic API capabilities**.
//！
//！ More documentation can be found at [Github-Kovi](https://github.com/Threkork/Kovi)


mod log;

pub use bot::plugin_builder::PluginBuilder;
pub use kovi_macros::plugin;

/// Everything about bots is inside
pub mod bot;
pub mod error;

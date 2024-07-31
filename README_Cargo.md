**English** | [简体中文](https://github.com/Threkork/Kovi/blob/main/README_zh.md)

# Kovi

A **fast and lightweight** OneBot V11 bot plugin framework developed with Rust.

The project is in beta status, currently featuring **message listening** and **basic API capabilities**.

Other features are under development.

**Note ⚠️, the project is in Beta status and the following may change**

**Note ⚠️, the project currently only supports OneBot V11 positive WebSocket protocol**

## Why Choose Kovi?

- 🚲 Lightweight: Low resource usage. So far, the compiled library size on Linux is less than 1.5MB.
- ⚡ Efficient: Due to its lightweight nature, it is fast enough, processing and delivering messages to plugins in less than 5 microseconds.
- 🚤 Rapid Development: Developers do not need to worry about underlying details, the framework handles everything for you.

The initial purpose of this project is to enhance group activity, facilitate group management, and is intended for personal entertainment, learning, and communication only. **No one is allowed to use this project for any illegal purposes.**

## Why is it called Kovi?

Because the bot plugin writing style is inspired by [Kivi](https://github.com/xiaotian2333/KiviBot-Primitive). If you have previously developed plugins for the Kivi framework, you will find it easy to get started with this framework.

## Getting Started

**Note ⚠️, the project is in Beta status and the following may change**

**Note ⚠️, the project currently only supports OneBot V11 positive WebSocket protocol**

The project is written in [Rust](#), and plugins also need to be written in [Rust](#), please make sure it is installed locally.

1. Create a basic rust project and add the framework.

```bash
cargo new kovi-bot
cd ./kovi-bot
cargo add Kovi
```

2. Create a bot instance in **src/main.rs**

```rust
use kovi::bot::Bot;
fn main() {
    let bot = Bot::build();
    bot.run()
}
```

If this is the first run, during **Bot::build()**, you will be prompted to enter some information to create the **kovi.conf.json** file, which is necessary for Kovi to run.

```
✔ What is the IP of the OneBot server? · 127.0.0.1
(Default: 127.0.0.1)

✔ What is the port of the OneBot server? · 8081
(Default: 8081)

✔ What is the access_token of the OneBot server? · 
(Default: Null)

✔ What is the ID of the main administrator? 
(No default value)
```


## Plugin Development

### Creating a Plugin

The recommended way to develop plugins is to create a new directory `plugins` to store them. Follow the steps below.

First, create a Cargo workspace, write `[workspace]` in `Cargo.toml`

```toml
[package]
...
[dependencies]
...

[workspace]
```

Then

```bash
cargo new plugins/hi --lib
```

Cargo will handle the rest for you.

### Writing Plugin

Edit the newly created plugin `plugins/hi/src/lib.rs`

Here is the minimal example

```rust
// Import the plugin builder structure
use kovi::bot::plugin_builder::PluginBuilder;

// The main function takes a mut plugin, which is necessary for mounting the plugin.
pub fn main(mut plugin: PluginBuilder) {
    // Set the plugin name; if the name is not set, all listeners will return error
    plugin.set_info("hi");

    // on_msg() listens for messages; event contains all information about the current message.
    plugin.on_msg(move |event| {
            if event.text == Option::Some("Hi Bot".to_string()) {
                event.reply("Hi!")
            }
            // Must return Ok(), currently has no effect; future versions will handle Err() accordingly
            Ok(())
        }) // As long as the name is set correctly, this will not return an error, so .unwrap() is fine
        .unwrap();
}
```


The main function is written in `lib.rs` because it needs to be exported for the bot instance to mount.

Plugins generally do not need `main.rs`.

### Mounting Plugins

Import the plugin into kovi-bot's main.rs

```bash
cargo add --path plugins/hi  
```

```rust
use kovi::bot::Bot;
use std::sync::Arc;

fn main() {
    let bot = Bot::build();
    let bot = bot
        .mount_main(Arc::new(hi::main))
        .mount_main(Arc::new(hi2::main))
        .mount_main(Arc::new(hi3::main));
    bot.run()
}

```

### More Plugin Examples

#### Bot Initiates Conversation

```rust
use kovi::bot::plugin_builder::PluginBuilder;

pub fn main(mut plugin: PluginBuilder) {
    plugin.set_info("online");
    // Construct RuntimeBot
    let bot = plugin.build_runtime_bot();
    let user_id = bot.main_admin;

    bot.send_private_msg(user_id, "bot online")
}
```

The `main()` function runs only once when KoviBot starts.
The closure passed to `plugin.on_msg()` runs every time a message is received.

Currently, the beta version of Kovi has not encapsulated more APIs. You can use `RuntimeBot` 's `api_tx` to send APIs manually.
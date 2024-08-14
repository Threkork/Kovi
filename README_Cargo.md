**English** | [简体中文](https://github.com/Threkork/Kovi/blob/main/README_zh.md)

# Kovi

A OneBot V11 bot plugin framework developed in Rust.

The project is currently in beta.

More features will be added in future updates.

**Note⚠️: The project is in Beta, and the following may change.**

**Note⚠️: The project currently only supports the OneBot V11 forward WebSocket protocol.**

## Why choose Kovi?

- 🚲 Lightweight: Low resource usage. So far, when compiled under Linux, the library size is less than 2MB.
- ⚡ Efficient: Because it's lightweight, it's also fast.
- 🚤 Rapid Development: Developers don't need to worry about the underlying details; the framework handles everything for you.

The original intent behind this project is to enhance group activity, facilitate group management, and is intended for personal entertainment, learning, and communication purposes only. **No one is allowed to use this project for any illegal activities.**

## Why is it called Kovi?

The bot plugin development method is derived from [Kivi](#). The [Kivi](#) repository is no longer available, but you can check out its author [Viki](https://github.com/vikiboss). If you've developed plugins for the [Kivi](#) framework before, getting started with this framework will be easy.

## Getting Started

**Note⚠️: The project is in Beta, and the following may change.**

**Note⚠️: The project currently only supports the OneBot V11 forward WebSocket protocol.**

The project is written in [Rust](#), and plugins also need to be written in [Rust](#). Please ensure that Rust is installed locally.

1. Create a basic Rust project and add the framework.

```bash
cargo new my-kovi-bot
cd ./my-kovi-bot
cargo add Kovi
```

2. Create a bot instance in **src/main.rs**

```rust
use kovi::build_bot;
fn main() {
    let bot = build_bot!();
    bot.run()
}
```

If this is your first run, during `build_bot`, you'll be prompted to enter some information to create the `kovi.conf.json` file, which is required for Kovi to run.

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

Then,

```bash
cargo new plugins/hi --lib
```

Cargo will take care of everything for you.

### Writing a Plugin

Write your newly created plugin in `plugins/hi/src/lib.rs`.

Here's a minimal example:

```rust
// Import the plugin builder structure
use kovi::PluginBuilder;

#[kovi::plugin] // Build the plugin
pub fn main(mut plugin: PluginBuilder) {
    // The main function must accept PluginBuilder, as it is the foundation of the plugin.

    plugin.on_msg(move |event| {
        // on_msg() listens for messages, and event contains all the information of the current message.
        if event.borrow_text() == Some("Hi Bot") {
            event.reply("Hi!") // Quick reply
        }
    });
}
```

The main function is written in `lib.rs` because it will be exported later to be mounted to the bot instance.

Plugins generally don't need a `main.rs`.

### Mounting the Plugin

Import the plugin into `my-kovi-bot`'s `main.rs`.

```bash
cargo add --path plugins/hi  
```

```rust
use kovi::build_bot;

fn main() {
    let bot = build_bot!(hi,hi2,plugin123);
    bot.run()
}
```

### More Plugin Examples

#### Bot Sending Messages Actively

```rust
use kovi::PluginBuilder;

#[kovi::plugin]
pub fn main(mut plugin: PluginBuilder) {
    // Build RuntimeBot
    let bot = plugin.build_runtime_bot();
    let user_id = bot.main_admin;

    bot.send_private_msg(user_id, "bot online")
}
```

The `main()` function runs only once when KoviBot starts.

The closure passed to `plugin.on_msg()` runs every time a message is received.

Kovi has encapsulated all available OneBot standard APIs. To extend the API, you can use `RuntimeBot`'s `api_tx` to send APIs yourself.

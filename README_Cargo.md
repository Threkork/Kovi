**English** | [简体中文](https://github.com/Threkork/Kovi/blob/main/README_zh.md)

# Kovi

A OneBot V11 bot plugin framework developed in Rust.

You can find more documentation in the [Kovi Doc](https://threkork.github.io/kovi-doc/).

The project is currently in beta.

More features will be added in future updates.

**Note⚠️: The project is in Beta, and the following may change.**

**Note⚠️: The project currently only supports the OneBot V11 forward WebSocket protocol.**

The original intent behind this project is to enhance group activity, facilitate group management, and is intended for personal entertainment, learning, and communication purposes only. **No one is allowed to use this project for any illegal activities.**

## Why is it called Kovi?

The bot plugin development method is derived from [Kivi](#). The [Kivi](#) repository is no longer available, but you can check out its author [Viki](https://github.com/vikiboss). If you've developed plugins for the [Kivi](#) framework before, getting started with this framework will be easy.

## Getting Started

The project is written in [Rust](#), and plugins also need to be written in [Rust](#). Please ensure that Rust is installed locally.

1. Create a basic Rust project and add the framework.

```bash
cargo install kovi-cli
cargo kovi new my-kovi-bot
cd ./my-kovi-bot
```

2. You will see that a bot instance has been generated in **src/main.rs**.

```rust
use kovi::build_bot;

fn main() {
    kovi::set_logger();
    let a = build_bot!();
    a.run()
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

Follow the steps below.

```bash
cargo kovi create hi
```

`kovi-cli` and `cargo` will take care of everything for you.

You will see that a new `plugins/hi` directory has been created. This is also the recommended way to develop plugins, as it’s always good to manage them in a directory.

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

```bash
cargo kovi add hi
```

Alternatively, you can use `cargo` directly; both are the same. This will add a local dependency in the root project’s `Cargo.toml`.

```bash
cargo add --path plugins/hi
```

```rust
use kovi::build_bot;

fn main() {
    kovi::set_logger();
    let bot = build_bot!(hi,hi2,plugin123);
    bot.run()
}
```

### More Plugin Examples

#### Bot Actively Sending Messages

```rust
use kovi::PluginBuilder;

#[kovi::plugin]
pub fn main(mut plugin: PluginBuilder) {
    // Build a RuntimeBot
    let bot = plugin.build_runtime_bot();
    let user_id = bot.main_admin;

    bot.send_private_msg(user_id, "bot online")
}
```

The `main()` function runs only once when KoviBot starts.

The closure passed to `plugin.on_msg()` runs every time a message is received.

Kovi has encapsulated all available OneBot standard APIs. To extend the API, you can use `RuntimeBot`'s `send_api()` to send APIs yourself.

You can find more documentation in the [Kovi Doc](https://threkork.github.io/kovi-doc/).

**English** | [简体中文](https://threkork.github.io/kovi-doc/)

# Kovi

A OneBot V11 bot plugin framework developed in Rust.

You can find more documentation in the [Kovi Doc](https://threkork.github.io/kovi-doc/).

The project is currently in beta.

More features will be added in future updates.

**Note⚠️: The project is in Beta, and the following may change.**

**Note⚠️: The project currently only supports the OneBot V11 forward WebSocket protocol.**

## Getting Started

It's recommended to use `kovi-cli` to manage your Kovi bot project.

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
    let bot = build_bot!();
    bot.run()
}
```

If this is your first run, during `build_bot`, you'll be prompted to enter some information to create the `kovi.conf.json` file, which is required for Kovi to run.

```
✔ What is the type of the host of the OneBot server? · IPv4

✔ What is the IP of the OneBot server? · 127.0.0.1
(Default: 127.0.0.1)

✔ What is the port of the OneBot server? · 8081
(Default: 8081)

✔ What is the access_token of the OneBot server? (Optional) ·
(Default: empty)

✔ What is the ID of the main administrator? (Not used yet)
(Optional)

✔ Do you want to view more optional options? · No
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
use kovi::PluginBuilder as plugin;

#[kovi::plugin] // Build the plugin
async fn main() {
    plugin::on_msg(|event| async move {
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
    let bot = build_bot!(hi,hi2,plugin123);
    bot.run()
}
```

### More Plugin Examples

#### Bot Actively Sending Messages

```rust
use kovi::PluginBuilder as plugin;

#[kovi::plugin]
async fn main() {
    // get a RuntimeBot
    let bot = plugin::get_runtime_bot();
    let user_id = bot.main_admin;

    bot.send_private_msg(user_id, "bot online")
}
```

The `main()` function runs only once when plugin starts.

The closure passed to `plugin::on_msg()` runs every time a message is received.

Kovi has encapsulated all available OneBot standard APIs. To extend the API, you can use `RuntimeBot`'s `send_api()` to send APIs yourself. You can check out the API extension plugins available for your needs at [Kovi Plugin Shop](https://threkork.github.io/kovi-doc/start/plugins).

You can find more documentation in the [Kovi Doc](https://threkork.github.io/kovi-doc/).

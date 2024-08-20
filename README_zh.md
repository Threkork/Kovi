<div align="center">

![Badge](https://img.shields.io/badge/OneBot-11-black) [![群](https://img.shields.io/badge/QQ%E7%BE%A4-857054777-54aeff)](https://qm.qq.com/q/kmpSBOVaCI)

[English](README.md) |  **简体中文** 

</div>

# Kovi

使用 Rust 开发的 OneBot V11 机器人插件框架。

你可以在 [Kovi使用手册](https://threkork.github.io/kovi-doc/) 看到更多文档

项目处于 beta 状态。

更多能力请等待开发。

**注意⚠️，项目处于 Beta 状态，以下可能会变动**

**注意⚠️，项目目前只支持 OneBot V11 正向 WebSocket 协议**

本项目开发初衷在于提高群活跃氛围、方便群管理，仅供个人娱乐、学习和交流使用，**任何人不得将本项目用于任何非法用途**。

## 为什么叫做Kovi

因为机器人插件写法来源于 [Kivi](#) , [Kivi](#) 的仓库已经不开放了，你可以看看它的作者 [Viki](https://github.com/vikiboss) 。如果你之前开发过 [Kivi](#) 框架的插件，对于上手本框架会很简单。

## 快速上手

**注意⚠️，项目处于 Beta 状态，以下可能会变动**

**注意⚠️，项目目前只支持 OneBot V11 正向 WebSocket 协议**

项目由 [Rust](#) 所写，插件也需用 [Rust](#) 写，请确保本地已安装。

1. 创建基本rust项目，加入框架。

```bash
cargo new my-kovi-bot
cd ./my-kovi-bot
cargo add Kovi
```

2. 在 **src/main.rs** 创建bot实例
```rust
use kovi::build_bot;
fn main() {
    let bot = build_bot!();
    bot.run()
}
```

如果是第一次运行，在 `build_bot` 时，会提示输入一些信息以创建 `kovi.conf.json` 文件，这是Kovi运行所需的信息。

```
✔ What is the IP of the OneBot server? · 127.0.0.1
OneBot服务端的IP是什么？ (默认值：127.0.0.1)

✔ What is the port of the OneBot server? · 8081
OneBot服务端的端口是什么？ (默认值：8081)

✔ What is the access_token of the OneBot server? · 
OneBot服务端的access_token是什么？ (默认值：空)

✔ What is the ID of the main administrator? 
管理员的ID是什么？ (无默认值)
```


## 插件开发

### 创建插件

推荐的插件开发方法是创建新目录 `plugins` 储存插件。跟着下面来吧。

首先创建 Cargo 工作区，在 `Cargo.toml` 写入 `[workspace]`

```toml
[package]
略
[dependencies]
略

[workspace]
```

接着

```bash
cargo new plugins/hi --lib
```

Cargo 会帮你做好一切的。

### 编写插件

编写我们新创建的插件 `plugins/hi/src/lib.rs`

下面是最小实例

```rust
// 导入插件构造结构体
use kovi::PluginBuilder;

#[kovi::plugin] //构造插件
pub fn main(mut plugin: PluginBuilder) {
    // 必须要求main传入 PluginBuilder ，这是插件的基础。
    
    plugin.on_msg(move |event| {
        // on_msg() 为监听消息，event 里面包含本次消息的所有信息。
        if event.borrow_text() == Some("Hi Bot") {
            event.reply("Hi!") //快捷回复
        }
    });
}
```

main函数写在 `lib.rs` 是因为等下要导出给bot实例挂载。

插件一般不需要 `main.rs`

### 挂载插件

将插件导入到 `my-kovi-bot` 的 `main.rs`

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

### 更多插件例子

#### bot 主动发言

```rust
use kovi::PluginBuilder;

#[kovi::plugin]
pub fn main(mut plugin: PluginBuilder) {
    // 构造RuntimeBot
    let bot = plugin.build_runtime_bot();
    let user_id = bot.main_admin;

    bot.send_private_msg(user_id, "bot online")
}
```

`main()` 函数 只会在 KoviBot 启动时运行一次。

向 `plugin.on_msg()` 传入的闭包，会在每一次接收消息时运行。

Kovi 已封装所有可用 OneBot 标准 api ，拓展 api 你可以使用 `RuntimeBot` 的 `send_api()` 来自行发送 api

你可以在 [Kovi使用手册](https://threkork.github.io/kovi-doc/) 看到更多文档
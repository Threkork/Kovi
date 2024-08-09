use chrono::Duration;

use log::info;

use super::{plugin_builder::Plugin, runtimebot::RuntimeBot, Bot};
use crate::bot::plugin_builder::PluginBuilder;
use std::{
    borrow::Borrow,
    process::exit,
    sync::{Arc, Mutex, RwLock},
    thread::sleep,
    time::Duration as StdDuratiom,
};

pub fn default_plugin_main(mut plugin: PluginBuilder, bot: Arc<RwLock<Bot>>) {
    plugin.set_info("Kovi default plugin");
    let time = Arc::new(Mutex::new(chrono::Local::now()));

    let get_plugin_list_str = move || -> String {
        let bot = bot.read().unwrap();
        let plugins: &Vec<Plugin> = bot.plugins.borrow();
        let mut str_vec = Vec::new();
        str_vec.push(String::from("〓 Kovi 插件列表 〓"));
        for plugin in plugins {
            str_vec.push(plugin.name.clone())
        }

        str_vec.join("\n")
    };


    let runtime_bot = plugin.build_runtime_bot();
    plugin
        .on_admin_msg(move |event| {
            let text = if let Some(text) = event.borrow_text() {
                text
            } else {
                return;
            };

            match text {
                ".h" | ".help" => {event.reply("〓 Kovi 帮助 〓.p 插件操作\n\n.s 框架状态\n.h 显示帮助\n.a 关于框架\n.e 退出进程")},
                ".p" | ".plugin" =>{event.reply("〓 Kovi 插件命令 〓\n.p list\n.p on/off <name?>")},
                ".p list" | ".plugin list" =>{event.reply(get_plugin_list_str())},
                ".s" | ".status" =>{event.reply(now_status(&runtime_bot,time.clone()))},
                ".a" | ".about" =>{event.reply("〓 关于 Kovi 〓\nKovi 是一个 OneBot 插件框架\n使用 Rust 所写\n开源地址：https://github.com/Threkork/Kovi")},
                ".e"|".exit"=>{event.reply("真的要关闭 KoviBot 吗？\n关闭请输入.exit -y")},
                ".e -y"|".exit -y"=>{event.reply("KoviBot 已关闭"); sleep(StdDuratiom::from_secs(1));kovi_exit()},
                _ => {}
            }
        })
        .unwrap();
}


fn now_status(
    runtime_bot: &RuntimeBot,
    time: Arc<Mutex<chrono::DateTime<chrono::Local>>>,
) -> String {
    let mut str_vec = Vec::new();
    let login_info = runtime_bot.get_login_info().unwrap();
    let name = login_info.get("nickname").unwrap().as_str().unwrap();
    let friends_binding = runtime_bot.get_friend_list().unwrap();
    let friends = friends_binding.as_array().unwrap().len();
    let groups_binding = runtime_bot.get_group_list().unwrap();
    let groups = groups_binding.as_array().unwrap().len();
    // 运行时间
    let now_time = chrono::Local::now();
    let time = time.lock().unwrap();
    let duration = now_time - *time;
    // chrono处理人类看的时间字符串
    let time_str = format_duration(duration);

    let version = env!("CARGO_PKG_VERSION");

    str_vec.push("〓 Kovi 框架实时状态 〓".to_string());
    str_vec.push(format!("昵称: {}", name));
    str_vec.push(format!("列表: {} 好友，{} 群", friends, groups));
    str_vec.push(format!("运行: {}", time_str));
    str_vec.push(format!("框架: v{}", version));

    str_vec.join("\n")
}

fn format_duration(duration: Duration) -> String {
    let seconds = duration.num_seconds();
    let minutes = seconds / 60;
    let hours = minutes / 60;
    let days = hours / 24;

    if days > 0 {
        format!("{}d {}h {}min", days, hours % 24, minutes % 60)
    } else if hours > 0 {
        format!("{}h {}min", hours, minutes % 60)
    } else {
        format!("{}min", minutes)
    }
}

fn kovi_exit() -> ! {
    info!("[Kovi] 通过消息命令主动结束运行程序");
    exit(1)
}

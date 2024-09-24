use super::RuntimeBot;
use crate::Bot;
use std::{
    path::PathBuf,
    sync::{Arc, RwLock},
};

pub trait KoviApi {
    /// 获取插件自己的路径
    fn get_data_path(&self) -> PathBuf;
    /// 获取 KoviBot 用来操控 KoviBot 自身。危险⚠️
    fn get_kovi_bot(&self) -> Arc<RwLock<Bot>>;
}

impl KoviApi for RuntimeBot {
    fn get_data_path(&self) -> PathBuf {
        let mut current_dir = std::env::current_dir().unwrap();

        current_dir.push(format!("data/{}", self.plugin_name));
        current_dir
    }

    fn get_kovi_bot(&self) -> Arc<RwLock<Bot>> {
        self.bot.clone()
    }
}

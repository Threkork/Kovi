use thiserror::Error;

#[derive(Error, Debug)]
pub enum MessageError {
    /// 解析出错
    #[error("Parse error: {0}")]
    ParseError(String),
    // #[error("Error, and no one knows why something went wrong")]
    // UnknownError(),
}

#[derive(Error, Debug)]
pub enum BotError {
    /// 没有寻找到插件
    #[error("Plugin not found: {0}")]
    PluginNotFound(String),
    #[error("Bot's Weak reference has expired")]
    RefExpired,
}

#[derive(Error, Debug)]
pub enum BotBuildError {
    /// 解析TOML文件失败
    #[error("Failed to parse TOML:\n{0}\nPlease reload the config file")]
    TomlParseError(String),
    /// 无法创建配置文件
    #[error("Failed to create config file: {0}")]
    FileCreateError(String),
    /// 无法读取TOML文件
    #[error("Failed to read TOML file: {0}")]
    FileReadError(String),
}

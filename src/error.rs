use thiserror::Error;

#[derive(Error, Debug)]
pub enum MessageError {
    //解析出错
    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Error, and no one knows why something went wrong")]
    UnknownError(),
}


#[derive(Error, Debug)]
pub enum BotError {
    //没有寻找到插件
    #[error("Plugin not found: {0}")]
    PluginNotFound(String),

    #[error("Error, and no one knows why something went wrong")]
    UnknownError(),
}

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    //解析出错
    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Error, and no one knows why something went wrong")]
    UnknownError(),
}

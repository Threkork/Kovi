use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("JSON serialization error: {0}")]
    JsonError(String),

    #[error("Error, and no one knows why something went wrong")]
    UnknownError(),
}

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Error, and no one knows why something went wrong")]
    UnknownError(),
}

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Check the incoming parameter: {0}")]
    ParamsError(String),

    #[error("Error, and no one knows why something went wrong")]
    UnknownError(),
}


#[derive(Error, Debug)]
pub enum PluginBuilderError {
    #[error("The information of the plugin is not set correctly")]
    InfoError(),
}

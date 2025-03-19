use super::{AccessControlMode, plugin_builder::listen::Listen};
use crate::{bot::runtimebot::kovi_api::AccessList, types::KoviAsyncFn};
use std::sync::Arc;
use tokio::sync::watch;

#[derive(Clone)]
pub struct DylibPlugin {
    pub(crate) enable_on_startup: bool,
    pub(crate) enabled: watch::Sender<bool>,

    pub name: String,
    pub version: String,
    pub(crate) main: Arc<KoviAsyncFn>,
    pub(crate) listen: Listen,

    #[cfg(feature = "plugin-access-control")]
    pub(crate) access_control: bool,
    #[cfg(feature = "plugin-access-control")]
    pub(crate) list_mode: AccessControlMode,
    #[cfg(feature = "plugin-access-control")]
    pub(crate) access_list: AccessList,
}

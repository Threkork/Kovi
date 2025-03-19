use crate::{ApiReturn, MsgEvent, NoticeEvent, RequestEvent, bot::SendApi};
use std::{pin::Pin, sync::Arc};
use tokio::sync::oneshot;

pub(crate) type KoviAsyncFn = dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync;

pub type PinFut = Pin<Box<dyn Future<Output = ()> + Send>>;

pub type MsgFn = Arc<dyn Fn(Arc<MsgEvent>) -> PinFut + Send + Sync>;

pub type NoticeFn = Arc<dyn Fn(Arc<NoticeEvent>) -> PinFut + Send + Sync>;

pub type RequestFn = Arc<dyn Fn(Arc<RequestEvent>) -> PinFut + Send + Sync>;

pub type NoArgsFn = Arc<dyn Fn() -> PinFut + Send + Sync>;

pub(crate) type ApiOneshotSender = oneshot::Sender<Result<ApiReturn, ApiReturn>>;
pub(crate) type ApiOneshotReceiver = oneshot::Receiver<Result<ApiReturn, ApiReturn>>;

pub(crate) type ApiAndOneshot = (
    SendApi,
    Option<oneshot::Sender<Result<ApiReturn, ApiReturn>>>,
);

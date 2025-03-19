use crate::types::{MsgFn, NoArgsFn, NoticeFn, RequestFn};
use std::sync::Arc;

#[derive(Clone, Default)]
pub(crate) struct Listen {
    pub(crate) msg: Vec<Arc<ListenMsgFn>>,
    #[cfg(feature = "message_sent")]
    pub(crate) msg_sent: Vec<MsgFn>,
    pub(crate) notice: Vec<NoticeFn>,
    pub(crate) request: Vec<RequestFn>,
    pub(crate) drop: Vec<NoArgsFn>,
}

#[derive(Clone)]
pub(crate) enum ListenMsgFn {
    Msg(MsgFn),
    PrivateMsg(MsgFn),
    GroupMsg(MsgFn),
    AdminMsg(MsgFn),
}

impl Listen {
    pub fn clear(&mut self) {
        self.msg.clear();
        self.notice.clear();
        self.request.clear();
        self.drop.clear();
        self.msg.shrink_to_fit();
        self.notice.shrink_to_fit();
        self.request.shrink_to_fit();
        self.drop.shrink_to_fit();
        #[cfg(feature = "message_sent")]
        self.msg_sent.clear();
        #[cfg(feature = "message_sent")]
        self.msg_sent.shrink_to_fit();
    }
}

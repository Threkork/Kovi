// use ahash::RandomState;
// use parking_lot::Mutex;
// use std::{collections::HashMap, sync::LazyLock};

// pub mod oneshot;

// static MPSC_MANAGER: LazyLock<Mutex<HashMap<String, Channel<Box<dyn Any + Send>, RandomState>>>> =
//     LazyLock::new(|| Mutex::new(HashMap::new()));

// pub(crate) struct MpscManaGer {}

// impl MpscManaGer {
//     pub(crate) fn init() -> Self {
//         todo!()
//     }
// }

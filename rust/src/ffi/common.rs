use std::sync::{Arc, LazyLock};
use tokio::runtime::Runtime;

pub(crate) static RUNTIME: LazyLock<Arc<Runtime>> = LazyLock::new(|| {
    Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to create Arrow runtime"),
    )
});

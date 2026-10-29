use std::sync::Once;

pub(super) static INIT: Once = Once::new();

pub(super) fn initialize() {
    INIT.call_once(|| {
        env_logger::init();
    });
}

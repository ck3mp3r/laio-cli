pub(crate) mod client;
pub(crate) mod layout;
pub(crate) mod model;
pub(crate) mod store;

#[cfg_attr(
    not(test),
    expect(
        unused_imports,
        reason = "unwired until the Muxer enum wiring subtask of parent 4b347a42; remove when wiring lands"
    )
)]
pub(crate) use layout::{LeafPlan, build_create_request};

#[cfg(test)]
#[path = "../../../test/muxer/rex/client_test.rs"]
mod client_test;

#[cfg(test)]
#[path = "../../../test/muxer/rex/layout_test.rs"]
mod layout_test;

#[cfg(test)]
#[path = "../../../test/muxer/rex/store_test.rs"]
mod store_test;

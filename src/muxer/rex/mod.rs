pub(crate) mod client;
pub(crate) mod export;
pub(crate) mod layout;
pub(crate) mod model;
pub(crate) mod mux;
pub(crate) mod store;

pub(crate) use export::{local_path, session_from_view, tiled_block_ids};
pub(crate) use layout::{LeafPlan, build_create_request};

pub(crate) use mux::Rex;

#[cfg(test)]
#[path = "../../../test/muxer/rex/client_test.rs"]
mod client_test;

#[cfg(test)]
#[path = "../../../test/muxer/rex/layout_test.rs"]
mod layout_test;

#[cfg(test)]
#[path = "../../../test/muxer/rex/store_test.rs"]
mod store_test;

#[cfg(test)]
#[path = "../../../test/muxer/rex/mux_test.rs"]
mod mux_test;

#[cfg(test)]
#[path = "../../../test/muxer/rex/mux_test/start.rs"]
mod mux_test_start;

#[cfg(test)]
#[path = "../../../test/muxer/rex/mux_test/switch.rs"]
mod mux_test_switch;

#[cfg(test)]
#[path = "../../../test/muxer/rex/mux_test/session.rs"]
mod mux_test_session;

#[cfg(test)]
#[path = "../../../test/muxer/rex/mux_test/stop.rs"]
mod mux_test_stop;

#[cfg(test)]
#[path = "../../../test/muxer/rex/export_test.rs"]
mod export_test;

#[cfg(test)]
#[path = "../../../test/muxer/rex/session_get_test.rs"]
mod session_get_test;

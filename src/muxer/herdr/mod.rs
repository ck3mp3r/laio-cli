pub(crate) mod client;
pub(crate) mod convert;
pub(crate) mod model;
pub(crate) mod mux;

pub(crate) use convert::{PaneOp, TabState, bind_pane_ops, build_plans, window_from_layout};
pub(crate) use mux::Herdr;

#[cfg(test)]
#[path = "../../../test/muxer/herdr/support.rs"]
mod support;

#[cfg(test)]
#[path = "../../../test/muxer/herdr/client_test.rs"]
mod client_test;

#[cfg(test)]
#[path = "../../../test/muxer/herdr/convert_test.rs"]
mod convert_test;

#[cfg(test)]
#[path = "../../../test/muxer/herdr/mux_test.rs"]
mod mux_test;

#[cfg(test)]
#[path = "../../../test/muxer/herdr/session_test.rs"]
mod session_test;

#[cfg(test)]
#[path = "../../../test/muxer/herdr/start_test.rs"]
mod start_test;

#[cfg(test)]
#[path = "../../../test/muxer/herdr/stop_sweep_test.rs"]
mod stop_sweep_test;

#[cfg(test)]
#[path = "../../../test/muxer/herdr/stop_test.rs"]
mod stop_test;

#[cfg(test)]
#[path = "../../../test/muxer/herdr/switch_test.rs"]
mod switch_test;

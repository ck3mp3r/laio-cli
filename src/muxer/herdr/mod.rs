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
#[path = "../../../test/muxer/herdr/client_test/protocol.rs"]
mod client_protocol_test;

#[cfg(test)]
#[path = "../../../test/muxer/herdr/client_test/connect.rs"]
mod client_connect_test;

#[cfg(test)]
#[path = "../../../test/muxer/herdr/client_test/workspace.rs"]
mod client_workspace_test;

#[cfg(test)]
#[path = "../../../test/muxer/herdr/client_test/metadata.rs"]
mod client_metadata_test;

#[cfg(test)]
#[path = "../../../test/muxer/herdr/client_test/layout_pane.rs"]
mod client_layout_pane_test;

#[cfg(test)]
#[path = "../../../test/muxer/herdr/client_test/tokens.rs"]
mod client_tokens_test;

#[cfg(test)]
#[path = "../../../test/muxer/herdr/client_test/server.rs"]
mod client_server_test;

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
#[path = "../../../test/muxer/herdr/start_test/lifecycle.rs"]
mod start_lifecycle_test;

#[cfg(test)]
#[path = "../../../test/muxer/herdr/start_test/provision.rs"]
mod start_provision_test;

#[cfg(test)]
#[path = "../../../test/muxer/herdr/start_test/behavior.rs"]
mod start_behavior_test;

#[cfg(test)]
#[path = "../../../test/muxer/herdr/start_test/support.rs"]
mod start_test_support;

#[cfg(test)]
#[path = "../../../test/muxer/herdr/stop_sweep_test/all.rs"]
mod stop_sweep_all_test;

#[cfg(test)]
#[path = "../../../test/muxer/herdr/stop_sweep_test/all_host_chain.rs"]
mod stop_sweep_all_host_chain_test;

#[cfg(test)]
#[path = "../../../test/muxer/herdr/stop_sweep_test/other.rs"]
mod stop_sweep_other_test;

#[cfg(test)]
#[path = "../../../test/muxer/herdr/stop_sweep_test/teardown.rs"]
mod stop_sweep_teardown_test;

#[cfg(test)]
#[path = "../../../test/muxer/herdr/stop_test/validation.rs"]
mod stop_validation_test;

#[cfg(test)]
#[path = "../../../test/muxer/herdr/stop_test/hooks.rs"]
mod stop_hooks_test;

#[cfg(test)]
#[path = "../../../test/muxer/herdr/stop_test/named.rs"]
mod stop_named_test;

#[cfg(test)]
#[path = "../../../test/muxer/herdr/stop_test/resolve.rs"]
mod stop_resolve_test;

#[cfg(test)]
#[path = "../../../test/muxer/herdr/switch_test.rs"]
mod switch_test;

pub(crate) mod client;
pub(crate) mod mux;
pub(crate) mod parser;
pub(crate) mod target;

pub(crate) use client::Dimensions;
pub(crate) use mux::Tmux;
pub(crate) use target::Target;

#[cfg(test)]
#[path = "../../../test/muxer/tmux/test/client.rs"]
mod tmux_client_test;

#[cfg(test)]
#[path = "../../../test/muxer/tmux/test/mux_start.rs"]
mod tmux_mux_start_test;

#[cfg(test)]
#[path = "../../../test/muxer/tmux/test/mux_stop_list.rs"]
mod tmux_mux_stop_list_test;

mod client;
mod model;
mod mux;

pub(crate) use mux::Zellij;

#[cfg(test)]
#[path = "../../../test/muxer/zellij/test.rs"]
mod test;

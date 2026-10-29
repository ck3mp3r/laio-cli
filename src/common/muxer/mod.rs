pub(crate) mod multiplexer;
pub(crate) use multiplexer::Multiplexer;
pub(crate) mod client;
pub(crate) use client::Client;

#[cfg(test)]
#[path = "../../../test/common/muxer/test.rs"]
pub(crate) mod test;

mod model;
pub mod template;
pub(crate) mod util;

pub mod variables;

pub(crate) use model::command::Command;
pub(crate) use model::flex_direction::FlexDirection;
pub(crate) use model::pane::Pane;
pub(crate) use model::script::Script;
pub(crate) use model::session::Session;
pub(crate) use model::window::Window;

#[cfg(test)]
#[path = "../../../test/common/config/template_test.rs"]
mod template_test;
#[cfg(test)]
#[path = "../../../test/common/config/variables_test.rs"]
mod variables_test;

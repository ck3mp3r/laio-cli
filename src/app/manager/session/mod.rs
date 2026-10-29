pub(crate) mod manager;

pub(crate) use manager::SessionManager;

#[cfg(test)]
#[path = "../../../../test/app/manager/session/test/stop.rs"]
mod session_stop_test;

#[cfg(test)]
#[path = "../../../../test/app/manager/session/test/start.rs"]
mod session_start_test;

#[cfg(test)]
#[path = "../../../../test/app/manager/session/test/variables.rs"]
mod session_variables_test;

#[cfg(test)]
#[path = "../../../../test/app/manager/session/test/list_yaml.rs"]
mod session_list_yaml_test;

#[cfg(test)]
#[path = "../../../../test/app/manager/session/test/support.rs"]
mod session_test_support;

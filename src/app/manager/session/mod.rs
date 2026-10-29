pub(crate) mod manager;

pub(crate) use manager::SessionManager;

#[cfg(test)]
#[path = "../../../../test/app/manager/session/test.rs"]
mod test;

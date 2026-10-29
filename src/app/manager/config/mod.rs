pub(crate) mod manager;

pub(crate) use manager::ConfigManager;

#[cfg(test)]
#[path = "../../../../test/app/manager/config/test.rs"]
mod test;

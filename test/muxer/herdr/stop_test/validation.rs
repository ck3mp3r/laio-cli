use miette::{Result, miette};

use crate::common::muxer::Multiplexer;

use super::mux::Herdr;

use super::support_api_mock::*;
use super::support_env::*;
use super::support_socket::*;

#[test]
fn stop_requires_a_name_outside_a_session() -> Result<()> {
    let _guard = env_lock()?;
    let previous = clear_herdr_env();

    let herdr = Herdr::new_with_runner(empty_runner(), MockHerdrApiMock::new());
    let result = herdr.stop(&None, &None, false, false, false);
    restore_env("HERDR_ENV", previous);

    let error = result
        .err()
        .ok_or_else(|| miette!("missing name outside a session should return Err"))?;

    assert_eq!(error.to_string(), "Specify laio session you want to stop.");
    Ok(())
}

#[test]
fn stop_other_requires_a_session() -> Result<()> {
    let _guard = env_lock()?;
    let previous = clear_herdr_env();

    let herdr = Herdr::new_with_runner(empty_runner(), MockHerdrApiMock::new());
    let result = herdr.stop(&None, &None, false, false, true);
    restore_env("HERDR_ENV", previous);

    let error = result
        .err()
        .ok_or_else(|| miette!("stop_other outside a session should return Err"))?;

    assert_eq!(error.to_string(), "Not inside a laio session.");
    Ok(())
}

#[test]
fn stop_rejects_all_or_other_with_a_name() -> Result<()> {
    let herdr = Herdr::new_with_runner(empty_runner(), MockHerdrApiMock::new());

    let all = herdr
        .stop(&Some("work".to_string()), &None, false, true, false)
        .err()
        .ok_or_else(|| miette!("stop_all with a name should return Err"))?;
    assert_eq!(
        all.to_string(),
        "Stopping all/other and specifying a session name are mutually exclusive."
    );

    let other = herdr
        .stop(&Some("work".to_string()), &None, false, false, true)
        .err()
        .ok_or_else(|| miette!("stop_other with a name should return Err"))?;
    assert_eq!(
        other.to_string(),
        "Stopping all/other and specifying a session name are mutually exclusive."
    );
    Ok(())
}

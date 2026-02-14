use crate::common::{
    cmd::{
        test::{MockCmdBoolMock, MockCmdStringMock, MockCmdUnitMock, RunnerMock},
        Type,
    },
    config::Session,
    muxer::Multiplexer,
};
use miette::{IntoDiagnostic, Result};
use serde_valid::{json::Value, yaml::FromYamlStr};

use super::Zellij;

#[test]
fn mux_start_session() -> Result<()> {
    let temp_dir = std::env::temp_dir();
    let temp_dir_lossy = temp_dir.to_string_lossy();
    let temp_dir_str = temp_dir_lossy.trim_end_matches('/');
    let yaml_str =
        include_str!("../../common/config/test/valid.yaml").replace("/tmp", temp_dir_str);
    let session = Session::from_yaml_str(&yaml_str).unwrap();

    let path_str = "./common/config/test/valid.yaml";

    let mut cmd_unit = MockCmdUnitMock::new();
    let mut cmd_string = MockCmdStringMock::new();
    let mut cmd_bool = MockCmdBoolMock::new();

    cmd_unit
        .expect_run()
        .times(1)
        .withf(
         {
            move |cmd|
            matches!(cmd,
              Type::Forget(_) if
              cmd.to_string().starts_with(&format!("LAIO_CONFIG={path_str} zellij --session valid --new-session-with-layout")))
          }
        )
        .returning(|_| Ok(()));

    cmd_bool
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "sh -c zellij list-sessions --short | grep \"valid\""))
        .returning(|_| Ok(false));

    cmd_string
        .expect_run()
        .times(2)
        .withf(
            |cmd| matches!(cmd, Type::Verbose(_) if ["date", "echo Hi"].contains(&cmd.to_string().as_str())),
        )
        .returning(|_| Ok("".to_string()));

    cmd_string
        .expect_run()
        .withf(|cmd| matches!(cmd, Type::Verbose(_) if cmd.to_string().contains("laio-277d3966f692fca8534baf09ce5fc483c928868d776993609681f6d524184281")))
        .returning(|_| Ok("".to_string()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool,
    };

    let zellij = Zellij::new_with_runner(runner);

    zellij.start(&session, path_str, false, false)?;

    Ok(())
}
#[test]
#[ignore = "flaky on GitHub Actions due to runner environment issues"]
fn mux_stop_session() -> Result<()> {
    let path_str = "./src/common/config/test/valid.yaml";

    let mut cmd_unit = MockCmdUnitMock::new();
    let mut cmd_string = MockCmdStringMock::new();
    let mut cmd_bool = MockCmdBoolMock::new();

    cmd_string
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "sh -c printenv ZELLIJ_SESSION_NAME || true"),
        )
        .returning(|_| Ok("valid".to_string()));

    cmd_string
        .expect_run()
        .times(2)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "sh -c printenv LAIO_CONFIG || true"),
        )
        .returning({
            move |_| Ok(path_str.to_string())
        });

    cmd_string
        .expect_run()
        .times(2)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "printenv ZELLIJ"))
        .returning(|_| Ok("0".to_string()));

    cmd_bool
        .expect_run()
        .times(2)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "sh -c zellij list-sessions --short | grep \"valid\""))
        .returning(|_| Ok(true));

    cmd_unit
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "zellij delete-session valid --force"))
        .returning(|_| Ok(()));

    cmd_string
        .expect_run()
        .times(2)
        .withf(|cmd| matches!(cmd, Type::Verbose(_) if ["date", "echo Bye"].contains(&cmd.to_string().as_str())))
        .returning(|_| Ok("".to_string()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool,
    };

    let zellij = Zellij::new_with_runner(runner);
    println!("DEBUG: About to call stop()");
    zellij.stop(&Some("valid".to_string()), &None, false, false, false)?;
    println!("DEBUG: stop() completed");

    Ok(())
}

#[test]
fn mux_get_session() -> Result<()> {
    let to_yaml = |yaml: String| -> Result<String> {
        let tmp_yaml: Value = serde_yaml::from_str(yaml.as_str()).into_diagnostic()?;
        let string_yaml = serde_yaml::to_string(&tmp_yaml).into_diagnostic()?;
        Ok(string_yaml)
    };

    let valid_yaml = to_yaml(include_str!("../../common/config/test/to_yaml.yaml").to_string())?;
    let valid_kdl = include_str!("../../common/config/test/to_yaml.kdl").to_string();

    let cmd_unit = MockCmdUnitMock::new();
    let mut cmd_string = MockCmdStringMock::new();
    let cmd_bool = MockCmdBoolMock::new();

    cmd_string
        .expect_run()
        .times(1)
        .withf(|cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "printenv ZELLIJ"))
        .returning(|_| Ok("0".to_string()));

    cmd_string
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "sh -c printenv ZELLIJ_SESSION_NAME || true"),
        )
        .returning(|_| Ok("valid".to_string()));

    cmd_string
        .expect_run()
        .times(1)
        .withf(
            |cmd| matches!(cmd, Type::Basic(_) if cmd.to_string() == "zellij action dump-layout"),
        )
        .returning(move |_| Ok(valid_kdl.to_string()));

    let runner = RunnerMock {
        cmd_unit,
        cmd_string,
        cmd_bool,
    };

    let zellij = Zellij::new_with_runner(runner);
    let result = zellij.get_session()?;

    let expected_session_yaml = to_yaml(serde_yaml::to_string(&result).into_diagnostic()?)?;
    assert_eq!(valid_yaml, expected_session_yaml);
    Ok(())
}

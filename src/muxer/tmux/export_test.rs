// Tests for the bash script export generator in `export.rs`.

use super::export::{generate_script, DEFAULT_HEIGHT, DEFAULT_WIDTH};
use super::Dimensions;
use crate::common::config::Session;
use serde_valid::yaml::FromYamlStr;

fn default_dimensions() -> Dimensions {
    Dimensions {
        width: DEFAULT_WIDTH,
        height: DEFAULT_HEIGHT,
    }
}

#[test]
fn export_minimal_session() {
    let yaml = r#"
name: minimal
path: /tmp
windows:
  - name: main
    panes:
      - flex: 1
"#;
    let session = Session::from_yaml_str(yaml).unwrap();
    let script = generate_script(&session, &default_dimensions());

    assert!(script.starts_with("#!/usr/bin/env bash"));
    assert!(script.contains("SESSION=\"minimal\""));
    assert!(script.contains("tmux new-session -d -s \"$SESSION\" -c \"/tmp\""));
    assert!(script.contains("tmux rename-window"));
    assert!(script.contains("tmux attach-session -t \"$SESSION\""));
    assert!(script.contains("set -euo pipefail"));
    assert!(script.contains("layout_checksum()"));
}

#[test]
fn export_session_with_env_and_shell() {
    let yaml = r#"
name: env-test
path: /home/user
shell: /bin/zsh
env:
  FOO: bar
  BAZ: qux
windows:
  - name: main
    panes:
      - flex: 1
"#;
    let session = Session::from_yaml_str(yaml).unwrap();
    let script = generate_script(&session, &default_dimensions());

    assert!(script.contains("-e \"FOO\"=\"bar\"") || script.contains("-e \"BAZ\"=\"qux\""));
    assert!(script.contains("tmux set-option -t \"$SESSION\" default-shell \"/bin/zsh\""));
}

#[test]
fn export_session_with_startup_commands() {
    let yaml = r#"
name: startup-test
path: /tmp
startup:
  - command: echo
    args: [Hello]
  - command: date
windows:
  - name: main
    panes:
      - flex: 1
"#;
    let session = Session::from_yaml_str(yaml).unwrap();
    let script = generate_script(&session, &default_dimensions());

    assert!(script.contains("# Startup commands"));
    assert!(script.contains("echo Hello"));
    assert!(script.contains("date"));
}

#[test]
fn export_session_with_shutdown_commands() {
    let yaml = r#"
name: shutdown-test
path: /tmp
shutdown:
  - command: echo
    args: [Bye]
windows:
  - name: main
    panes:
      - flex: 1
"#;
    let session = Session::from_yaml_str(yaml).unwrap();
    let script = generate_script(&session, &default_dimensions());

    assert!(script.contains("# Shutdown hook"));
    assert!(script.contains("echo Bye"));
    assert!(script.contains("session-closed"));
}

#[test]
fn export_multiple_windows() {
    let yaml = r#"
name: multi-win
path: /tmp
windows:
  - name: editor
    panes:
      - flex: 1
  - name: server
    panes:
      - flex: 1
  - name: logs
    panes:
      - flex: 1
"#;
    let session = Session::from_yaml_str(yaml).unwrap();
    let script = generate_script(&session, &default_dimensions());

    assert!(script.contains("# -- Window: editor --"));
    assert!(script.contains("# -- Window: server --"));
    assert!(script.contains("# -- Window: logs --"));
    assert!(script.contains("tmux rename-window"));
    assert!(script.contains("tmux new-window -Pd"));
}

#[test]
fn export_pane_with_commands() {
    let yaml = r#"
name: cmd-test
path: /tmp
windows:
  - name: main
    panes:
      - flex: 1
        commands:
          - command: echo
            args: [hello]
          - command: ls
            args: [-la]
"#;
    let session = Session::from_yaml_str(yaml).unwrap();
    let script = generate_script(&session, &default_dimensions());

    assert!(script.contains("tmux send-keys"));
    assert!(script.contains("echo hello"));
    assert!(script.contains("ls -la"));
}

#[test]
fn export_multiple_panes_column() {
    let yaml = r#"
name: column-test
path: /tmp
windows:
  - name: main
    flex_direction: column
    panes:
      - flex: 1
      - flex: 2
      - flex: 1
"#;
    let session = Session::from_yaml_str(yaml).unwrap();
    let dims = Dimensions {
        width: 160,
        height: 90,
    };
    let script = generate_script(&session, &dims);

    assert!(script.contains("PANE_0="));
    assert!(script.contains("PANE_1=$(tmux split-window"));
    assert!(script.contains("PANE_2=$(tmux split-window"));
    assert!(script.contains("LAYOUT="));
    assert!(script.contains("layout_checksum"));
    assert!(script.contains("select-layout"));
    let layout_line = script
        .lines()
        .find(|l| l.starts_with("LAYOUT="))
        .unwrap();
    assert!(layout_line.contains('['));
}

#[test]
fn export_multiple_panes_row() {
    let yaml = r#"
name: row-test
path: /tmp
windows:
  - name: main
    flex_direction: row
    panes:
      - flex: 1
      - flex: 1
"#;
    let session = Session::from_yaml_str(yaml).unwrap();
    let dims = Dimensions {
        width: 160,
        height: 90,
    };
    let script = generate_script(&session, &dims);

    let layout_line = script
        .lines()
        .find(|l| l.starts_with("LAYOUT="))
        .expect("LAYOUT line not found");
    assert!(layout_line.contains('{'));
    assert!(layout_line.contains('}'));
}

#[test]
fn export_nested_panes() {
    let yaml = r#"
name: nested-test
path: /tmp
windows:
  - name: main
    flex_direction: column
    panes:
      - flex: 1
        flex_direction: row
        panes:
          - flex: 1
          - flex: 2
      - flex: 1
"#;
    let session = Session::from_yaml_str(yaml).unwrap();
    let dims = Dimensions {
        width: 160,
        height: 90,
    };
    let script = generate_script(&session, &dims);

    assert!(script.contains("PANE_0="));
    assert!(script.contains("PANE_1="));
    assert!(script.contains("PANE_2="));

    let layout_line = script
        .lines()
        .find(|l| l.starts_with("LAYOUT="))
        .unwrap();
    assert!(layout_line.contains('['));
    assert!(layout_line.contains('{'));
}

#[test]
fn export_pane_properties() {
    let yaml = r#"
name: props-test
path: /tmp
windows:
  - name: main
    panes:
      - flex: 1
        name: my-pane
        style: bg=red,fg=default
"#;
    let session = Session::from_yaml_str(yaml).unwrap();
    let script = generate_script(&session, &default_dimensions());

    assert!(script.contains("-T \"my-pane\""));
    assert!(script.contains("-P \"bg=red,fg=default\""));
}

#[test]
fn export_pane_zoom_and_focus() {
    let yaml = r#"
name: zoom-focus-test
path: /tmp
windows:
  - name: main
    flex_direction: column
    panes:
      - flex: 1
        zoom: true
      - flex: 1
        focus: true
"#;
    let session = Session::from_yaml_str(yaml).unwrap();
    let script = generate_script(&session, &default_dimensions());

    assert!(script.contains("tmux resize-pane -Z"));
    assert!(script.contains("tmux select-pane -t"));
}

#[test]
fn export_pane_with_path() {
    let yaml = r#"
name: path-test
path: /home/user
windows:
  - name: main
    flex_direction: column
    panes:
      - flex: 1
        path: src
      - flex: 1
        path: /opt/logs
"#;
    let session = Session::from_yaml_str(yaml).unwrap();
    let script = generate_script(&session, &default_dimensions());

    assert!(script.contains("/home/user/src"));
    assert!(script.contains("/opt/logs"));
}

#[test]
fn export_pane_cmd_delay() {
    let yaml = r#"
name: delay-test
path: /tmp
pane_cmd_delay: 500
windows:
  - name: main
    panes:
      - flex: 1
"#;
    let session = Session::from_yaml_str(yaml).unwrap();
    let script = generate_script(&session, &default_dimensions());

    assert!(script.contains("sleep 0.5"));
}

#[test]
fn export_session_guard() {
    let yaml = r#"
name: guard-test
path: /tmp
windows:
  - name: main
    panes:
      - flex: 1
"#;
    let session = Session::from_yaml_str(yaml).unwrap();
    let script = generate_script(&session, &default_dimensions());

    assert!(script.contains("tmux has-session -t \"$SESSION\" 2>/dev/null"));
    assert!(script.contains("exit 0"));
}

#[test]
fn export_window_with_path() {
    let yaml = r#"
name: win-path-test
path: /home/user
windows:
  - name: api
    path: ./api
    panes:
      - flex: 1
  - name: web
    path: /opt/web
    panes:
      - flex: 1
"#;
    let session = Session::from_yaml_str(yaml).unwrap();
    let script = generate_script(&session, &default_dimensions());

    assert!(script.contains("/home/user/api"));
    assert!(script.contains("/opt/web"));
}

#[test]
fn export_empty_pane_window() {
    let yaml = r#"
name: empty-pane-test
path: /tmp
windows:
  - name: main
  - name: secondary
    panes:
      - flex: 1
"#;
    let session = Session::from_yaml_str(yaml).unwrap();
    let script = generate_script(&session, &default_dimensions());

    assert!(script.contains("# -- Window: main --"));
    assert!(script.contains("# -- Window: secondary --"));
}

#[test]
fn export_pane_script() {
    let yaml = r#"
name: script-test
path: /tmp
windows:
  - name: main
    panes:
      - flex: 1
        script: |
          #!/usr/bin/env bash
          echo "hello from script"
"#;
    let session = Session::from_yaml_str(yaml).unwrap();
    let script = generate_script(&session, &default_dimensions());

    assert!(script.contains("mktemp"));
    assert!(script.contains("chmod +x"));
    assert!(script.contains("echo \"hello from script\""));
    assert!(script.contains("LAIO_PANE_SCRIPT"));
}

#[test]
fn export_startup_script() {
    let yaml = r#"
name: startup-script-test
path: /tmp
startup_script: |
  #!/usr/bin/env bash
  echo "startup script"
windows:
  - name: main
    panes:
      - flex: 1
"#;
    let session = Session::from_yaml_str(yaml).unwrap();
    let script = generate_script(&session, &default_dimensions());

    assert!(script.contains("LAIO_STARTUP_SCRIPT"));
    assert!(script.contains("echo \"startup script\""));
}

#[test]
fn export_valid_yaml_matches_layout() {
    let temp_dir = std::env::temp_dir();
    let temp_dir_lossy = temp_dir.to_string_lossy();
    let temp_dir_str = temp_dir_lossy.trim_end_matches('/');
    let yaml_str =
        include_str!("../../common/config/test/valid.yaml").replace("/tmp", temp_dir_str);
    let session = Session::from_yaml_str(&yaml_str).unwrap();

    let dims = Dimensions {
        width: 160,
        height: 90,
    };
    let script = generate_script(&session, &dims);

    assert!(script.contains("PANE_0="));
    assert!(script.contains("PANE_1="));
    assert!(script.contains("PANE_2="));

    assert!(script.contains("# -- Window: code --"));
    assert!(script.contains("# -- Window: infrastructure --"));

    assert!(script.contains("-T \"foo\""));
    assert!(script.contains("tmux resize-pane -Z"));
    assert!(script.contains("tmux select-pane -t"));
    assert!(script.contains("bg=red,fg=default"));
    assert!(script.contains("date"));
    assert!(script.contains("echo Hi"));
    assert!(script.contains("echo \"hello again\""));
    assert!(script.contains("FOO"));
    assert!(script.contains("bar"));
}

#[test]
fn export_nats_pane_demo() {
    let yaml = r#"
name: nats-pane-demo
path: ~/
windows:
  - name: nats
    flex_direction: column
    panes:
      - name: nats-server
        flex: 60
        commands:
          - command: nats-server
      - flex_direction: row
        flex: 40
        panes:
          - name: sub
            flex: 1
            commands:
              - command: nats
                args: [sub, test]
          - name: one
            flex: 1
            commands:
              - command: nats
                args: [pub, test, one]
          - name: two
            flex: 1
            commands:
              - command: nats
                args: [pub, test, two]
"#;
    let session = Session::from_yaml_str(yaml).unwrap();
    let script = generate_script(&session, &default_dimensions());

    assert!(script.contains("PANE_0="));
    assert!(script.contains("PANE_1="));
    assert!(script.contains("PANE_2="));
    assert!(script.contains("PANE_3="));

    assert!(script.contains("-T \"nats-server\""));
    assert!(script.contains("-T \"sub\""));
    assert!(script.contains("-T \"one\""));
    assert!(script.contains("-T \"two\""));

    assert!(script.contains("nats-server"));
    assert!(script.contains("nats sub test"));
    assert!(script.contains("nats pub test one"));
    assert!(script.contains("nats pub test two"));
}

#[test]
fn export_layout_dimensions_match() {
    let yaml = r#"
name: dim-test
path: /tmp
windows:
  - name: main
    flex_direction: column
    panes:
      - flex: 1
      - flex: 1
"#;
    let session = Session::from_yaml_str(yaml).unwrap();
    let dims = Dimensions {
        width: 160,
        height: 90,
    };
    let script = generate_script(&session, &dims);

    let layout_line = script
        .lines()
        .find(|l| l.starts_with("LAYOUT="))
        .unwrap();
    assert!(layout_line.contains("160x90,0,0"));
}

#[test]
fn export_custom_dimensions() {
    let yaml = r#"
name: custom-dim
path: /tmp
windows:
  - name: main
    flex_direction: row
    panes:
      - flex: 1
      - flex: 1
"#;
    let session = Session::from_yaml_str(yaml).unwrap();
    let dims = Dimensions {
        width: 300,
        height: 80,
    };
    let script = generate_script(&session, &dims);

    let layout_line = script
        .lines()
        .find(|l| l.starts_with("LAYOUT="))
        .unwrap();
    assert!(layout_line.contains("300x80,0,0"));
}

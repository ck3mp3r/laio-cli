use std::{
    env, fs,
    io::{BufRead, BufReader, ErrorKind, Write},
    os::unix::net::UnixListener,
    path::{Path, PathBuf},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use miette::{IntoDiagnostic, Result, miette};
use serde_json::Value;

use crate::common::cmd::test::{MockCmdBoolMock, MockCmdStringMock, MockCmdUnitMock, RunnerMock};

pub(super) fn temp_socket_path(label: &str) -> PathBuf {
    env::temp_dir().join(format!("laio-herdr-{label}-{}.sock", std::process::id()))
}

pub(super) fn bind_listener(path: &Path) -> Result<UnixListener> {
    let _ = fs::remove_file(path);
    UnixListener::bind(path).into_diagnostic()
}

pub(super) fn spawn_responder(
    listener: UnixListener,
    response: String,
) -> JoinHandle<Result<String>> {
    thread::spawn(move || -> Result<String> {
        let (mut stream, _) = listener.accept().into_diagnostic()?;
        let mut reader = BufReader::new(stream.try_clone().into_diagnostic()?);
        let mut request = String::new();
        reader.read_line(&mut request).into_diagnostic()?;
        stream.write_all(response.as_bytes()).into_diagnostic()?;
        stream.flush().into_diagnostic()?;
        Ok(request)
    })
}

pub(super) fn spawn_canned_server(
    listener: UnixListener,
    responses: Vec<String>,
) -> JoinHandle<Result<Vec<String>>> {
    thread::spawn(move || -> Result<Vec<String>> {
        listener.set_nonblocking(true).into_diagnostic()?;
        let mut requests = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(2);

        while requests.len() < responses.len() && Instant::now() < deadline {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    stream.set_nonblocking(false).into_diagnostic()?;
                    let mut reader = BufReader::new(stream.try_clone().into_diagnostic()?);
                    let mut request = String::new();
                    let read = reader.read_line(&mut request).into_diagnostic()?;
                    if read == 0 {
                        continue;
                    }
                    stream
                        .write_all(responses[requests.len()].as_bytes())
                        .into_diagnostic()?;
                    stream.flush().into_diagnostic()?;
                    requests.push(request);
                }
                Err(error) if error.kind() == ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(2));
                }
                Err(error) => return Err(error).into_diagnostic(),
            }
        }

        Ok(requests)
    })
}

pub(super) fn join_thread<T>(handle: JoinHandle<Result<T>>) -> Result<T> {
    handle
        .join()
        .map_err(|_| miette!("responder thread panicked"))?
}

pub(super) fn parse_request(line: &str) -> Result<Value> {
    serde_json::from_str(line.trim_end()).into_diagnostic()
}

pub(super) fn empty_runner() -> RunnerMock {
    RunnerMock {
        cmd_unit: MockCmdUnitMock::new(),
        cmd_string: MockCmdStringMock::new(),
        cmd_bool: MockCmdBoolMock::new(),
    }
}

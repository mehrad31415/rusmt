use std::fmt::{Display, Formatter};
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime};
use std::{fs, thread};

use command_group::CommandGroup;
use log::{debug, warn};

use crate::backend::codegen::CodeGen;
use crate::backend::error::BackendResult;
use crate::ir::ctxt::IRContext;

/// This defines the name of the <file>.smt2
const FILE: &str = "main";

/// Execution timeout for the backend in seconds: by default 10 minutes (600 seconds).
const BACKEND_TIMEOUT: Duration = Duration::from_secs(60 * 10);

/// The response returned by the solver or other backend.
pub enum Response {
    /// solver does not return anything
    Timeout,
    /// solver explicitly returns the unknown status
    Unknown,
    /// SMT: sat
    Sat,
    /// SMT: unsat
    Unsat,
}

impl Display for Response {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::Timeout => "timeout",
            Self::Unknown => "unknown",
            Self::Sat => "sat",
            Self::Unsat => "unsat",
        };
        f.write_str(text)
    }
}

/// Entrypoint for generating code via the `backend` and executing it in an external process.
///
/// # Arguments
///
/// * `ir` - A reference to the intermediate representation to be translated into backend code.
/// * `backend` - A trait object that knows how to generate code for a specific solver.
/// * `path_wks` - The working directory in which the code and build files will be placed.
///
/// # Returns
///
/// A `BackendResult<Response>` indicating the outcome: `sat`, `unsat`, `unknown`, or `timeout`.
pub fn invoke_backend(
    ir: &IRContext,
    backend: &dyn CodeGen,
    path_wks: &Path,
) -> BackendResult<Response> {
    // 1. Generate SMTLIB2 source code from the IR using the backend's process method.
    let code = backend.process(ir)?;

    // 2. save source code to a file named `main.smt2`.
    let path_src = path_wks.join(format!("{}.{}", FILE, backend.flavor()));
    if path_src.exists() {
        panic!("source file already exists");
    }
    fs::write(&path_src, code).unwrap_or_else(|e| panic!("IO error on source file: {}", e));

    // 3. Invoke Z3 directly on the generated SMTLIB2 file.
    let mut cmd = Command::new("z3");
    cmd.arg("-smt2")
        .arg(&path_src)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.group_spawn().expect("spawning for execution");

    let mut stdout = child.inner().stdout.take().expect("piped stdout");
    let mut stderr = child.inner().stderr.take().expect("piped stderr");
    let timestamp = SystemTime::now();

    // monitor the execution
    let thread = thread::spawn(move || {
        loop {
            // print any on-going messages
            let mut message = String::new();
            stderr.read_to_string(&mut message).expect("reading stderr");
            if !message.is_empty() {
                debug!("{}", message);
            }

            // check status
            if let Ok(Some(status)) = child.try_wait() {
                // print any remaining messages
                let mut message = String::new();
                stderr.read_to_string(&mut message).expect("reading stderr");
                if !message.is_empty() {
                    debug!("{}", message);
                }
                return Some(status);
            }

            // check timeout
            if timestamp.elapsed().expect("time measurement") > BACKEND_TIMEOUT {
                child
                    .kill()
                    .expect("terminate the entire child process group");
                return None;
            }

            // wait a bit longer
            thread::sleep(Duration::from_millis(200));
        }
    });

    // wait for thread to finish
    let status = thread.join().expect("monitoring thread completed");

    // 5. Read Z3's output.
    let mut output = String::new();
    stdout.read_to_string(&mut output).expect("reading stdout");

    // 6. Interpret the output.
    let response = match status {
        None => {
            if !output.is_empty() {
                warn!("output received from a timeout execution: {}", output);
            }
            Response::Timeout
        }
        Some(e) => {
            if !e.success() {
                if !output.is_empty() {
                    warn!("output received from a crashed execution: {}", output);
                }
                panic!("backend execution crashed with status {}", e);
            }
            match output.trim() {
                "unknown" => Response::Unknown,
                "sat" => Response::Sat,
                "unsat" => Response::Unsat,
                other => panic!("invalid response: {}", other),
            }
        }
    };

    // finally, return the output
    Ok(response)
}

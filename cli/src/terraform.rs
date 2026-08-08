use crate::logger::Logger;
use miette::{IntoDiagnostic, Result};
use std::borrow::Cow;
use std::ffi::OsStr;
use std::io::{self, Read};
use std::os::fd::AsFd;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use wait_timeout::ChildExt;

pub const INIT_STATE_FILE: &str = ".terraform/terraform.tfstate";
pub const DEFAULT_READ_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Copy)]
pub struct CommandTimeout {
    duration: Duration,
    started: Instant,
}

impl CommandTimeout {
    pub fn new(duration: Duration) -> Self {
        Self {
            duration,
            started: Instant::now(),
        }
    }

    fn remaining(self) -> Duration {
        self.duration.saturating_sub(self.started.elapsed())
    }
}

pub fn command<I, S>(tf_bin: &Path, output_dir: &Path, args: I) -> Command
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut process = Command::new(tf_bin);
    process
        .current_dir(output_dir)
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    process
}

pub fn init_command(tf_bin: &Path, output_dir: &Path, global_args: &[String]) -> Result<Command> {
    // Keep requested command output isolated on stdout while retaining init prompts.
    let stderr = io::stderr()
        .as_fd()
        .try_clone_to_owned()
        .into_diagnostic()?;
    let mut process = command(tf_bin, output_dir, global_args);
    process.arg("init").stdout(Stdio::from(stderr));
    Ok(process)
}

pub fn subcommand(args: &[String]) -> Option<(usize, &str)> {
    args.iter()
        .enumerate()
        .find(|(_, argument)| !argument.starts_with('-'))
        .map(|(index, argument)| (index, argument.as_str()))
}

pub fn timeout(args: &[String], explicit: Option<Duration>) -> Option<Duration> {
    explicit.or_else(|| reads_remote_state(args).then_some(DEFAULT_READ_TIMEOUT))
}

pub fn read_timeout(explicit: Option<Duration>) -> Duration {
    explicit.unwrap_or(DEFAULT_READ_TIMEOUT)
}

fn reads_remote_state(args: &[String]) -> bool {
    let Some((index, command)) = subcommand(args) else {
        return false;
    };
    match command {
        "output" => true,
        "show" => subcommand(&args[index + 1..]).is_none(),
        "state" => matches!(
            subcommand(&args[index + 1..]).map(|(_, command)| command),
            Some("list" | "pull" | "show")
        ),
        "workspace" => matches!(
            subcommand(&args[index + 1..]).map(|(_, command)| command),
            Some("list" | "show")
        ),
        _ => false,
    }
}

pub fn initialized(output_dir: &Path, global_args: &[String]) -> bool {
    if let Some(path) = global_args
        .iter()
        .find_map(|argument| argument.strip_prefix("-chdir="))
    {
        return output_dir.join(path).join(INIT_STATE_FILE).is_file();
    }
    output_dir.join(INIT_STATE_FILE).is_file()
}

pub fn run_in(
    logger: &Logger,
    tf_bin: &Path,
    output_dir: &Path,
    args: &[&str],
    timeout: Option<Duration>,
) -> Result<ExitStatus> {
    run_with_timeout(
        logger,
        &mut command(tf_bin, output_dir, args),
        timeout.map(CommandTimeout::new),
    )
}

pub fn output_in(
    logger: &Logger,
    tf_bin: &Path,
    output_dir: &Path,
    args: &[&str],
    timeout: Option<Duration>,
) -> Result<Output> {
    let mut command = command(tf_bin, output_dir, args);
    command.stdin(Stdio::null()).stdout(Stdio::piped());
    output(logger, &mut command, timeout.map(CommandTimeout::new))
}

pub fn capture_in(
    logger: &Logger,
    tf_bin: &Path,
    output_dir: &Path,
    args: &[&str],
    timeout: Option<Duration>,
) -> Result<Output> {
    let mut command = command(tf_bin, output_dir, args);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    output(logger, &mut command, timeout.map(CommandTimeout::new))
}

pub fn run(logger: &Logger, command: &mut Command) -> Result<ExitStatus> {
    log_command(logger, command);
    command.status().into_diagnostic()
}

pub fn run_with_timeout(
    logger: &Logger,
    command: &mut Command,
    timeout: Option<CommandTimeout>,
) -> Result<ExitStatus> {
    let Some(timeout) = timeout else {
        return run(logger, command);
    };
    let command_string = log_command(logger, command);
    command.process_group(0);
    let mut child = command.spawn().into_diagnostic()?;
    let Some(status) = child.wait_timeout(timeout.remaining()).into_diagnostic()? else {
        kill_process_group(&mut child)?;
        child.wait().into_diagnostic()?;
        return Err(miette::miette!(
            "Command timed out after {}: {command_string}",
            humantime::format_duration(timeout.duration)
        ));
    };
    Ok(status)
}

pub fn output(
    logger: &Logger,
    command: &mut Command,
    timeout: Option<CommandTimeout>,
) -> Result<Output> {
    let Some(timeout) = timeout else {
        log_command(logger, command);
        return command.output().into_diagnostic();
    };
    let command_string = log_command(logger, command);
    command.process_group(0);
    let mut child = command.spawn().into_diagnostic()?;
    let stdout = child.stdout.take().map(|mut stdout| {
        thread::spawn(move || {
            let mut output = Vec::new();
            stdout.read_to_end(&mut output).map(|_| output)
        })
    });
    let stderr = child.stderr.take().map(|mut stderr| {
        thread::spawn(move || {
            let mut output = Vec::new();
            stderr.read_to_end(&mut output).map(|_| output)
        })
    });
    let (status, timed_out) =
        if let Some(status) = child.wait_timeout(timeout.remaining()).into_diagnostic()? {
            (status, false)
        } else {
            kill_process_group(&mut child)?;
            (child.wait().into_diagnostic()?, true)
        };
    let stdout = join_output(stdout)?;
    let stderr = join_output(stderr)?;
    if timed_out {
        return Err(miette::miette!(
            "Command timed out after {}: {command_string}",
            humantime::format_duration(timeout.duration)
        ));
    }
    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

fn kill_process_group(child: &mut Child) -> Result<()> {
    let process_group = i32::try_from(child.id())
        .map_err(|_| miette::miette!("Child process ID does not fit in a Unix process ID"))?;
    // Negative PIDs address process groups. The child becomes its own group leader before spawn.
    if unsafe { libc::kill(-process_group, libc::SIGKILL) } == -1 {
        let error = io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::ESRCH) {
            return Err(error).into_diagnostic();
        }
    }
    Ok(())
}

fn join_output(handle: Option<thread::JoinHandle<io::Result<Vec<u8>>>>) -> Result<Vec<u8>> {
    let Some(handle) = handle else {
        return Ok(Vec::new());
    };
    handle
        .join()
        .map_err(|_| miette::miette!("Command output reader panicked"))?
        .into_diagnostic()
}

fn command_string(command: &Command) -> String {
    let program = command.get_program().to_string_lossy();
    let args = command.get_args().map(OsStr::to_string_lossy);
    shell_words::join(std::iter::once(program).chain(args))
}

fn log_command(logger: &Logger, command: &Command) -> String {
    let command_string = command_string(command);
    info!(
        logger,
        "From: {}\n   Running: {}",
        command
            .get_current_dir()
            .map_or(Cow::from("<None>"), Path::to_string_lossy),
        command_string
    );
    command_string
}

#[cfg(test)]
mod tests {
    use super::{CommandTimeout, output, run_with_timeout, timeout};
    use crate::logger::Logger;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    #[test]
    fn test_terraform_timeout_defaults_remote_reads() {
        let expected = Duration::from_secs(30);

        assert_eq!(timeout(&["output".to_owned()], None), Some(expected));
        assert_eq!(timeout(&["apply".to_owned()], None), None);
        assert_eq!(
            timeout(&["apply".to_owned()], Some(expected)),
            Some(expected)
        );
    }

    #[test]
    fn test_run_with_timeout_kills_process_group() {
        let mut command = Command::new("sh");
        command.args(["-c", "sleep 30 & wait"]);
        let started = Instant::now();

        let error = run_with_timeout(
            &Logger::new(false),
            &mut command,
            Some(CommandTimeout::new(Duration::from_millis(50))),
        )
        .expect_err("command should time out");

        assert!(error.to_string().contains("timed out"));
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn test_output_timeout_kills_descendant_holding_pipe() {
        let mut command = Command::new("sh");
        command
            .args(["-c", "sleep 30 & wait"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let started = Instant::now();

        let error = output(
            &Logger::new(false),
            &mut command,
            Some(CommandTimeout::new(Duration::from_millis(50))),
        )
        .expect_err("command should time out");

        assert!(error.to_string().contains("timed out"));
        assert!(started.elapsed() < Duration::from_secs(5));
    }
}

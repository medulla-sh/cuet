use crate::cli::{CueCommand, Env};
use crate::logger::Logger;
use crate::workspace::Workspace;
use miette::{IntoDiagnostic, Result};
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

const OUTPUT_FOLDER_NAME: &str = ".cuet";
const OUTPUT_FILE_NAME: &str = "main.tf.json";

fn metadata_expression(workspace: &Workspace, backend_override_value: &str) -> String {
    let module = serde_json::Value::String(workspace.module_name().to_owned());
    format!(
        "{{ #metadata: {{ module: {module}, localBackendOverride: {backend_override_value} }} }}"
    )
}

fn export_expression(workspace: &Workspace, env: &Env, backend_override_value: &str) -> String {
    let environment = serde_json::Value::String(env.clone());
    format!(
        "((infra & {}).out)[{environment}]",
        metadata_expression(workspace, backend_override_value)
    )
}

fn cue_command(
    cue_bin: &Path,
    workspace: &Workspace,
    env: &Env,
    backend_override_value: &str,
    command: &CueCommand,
) -> Command {
    let (subcommand, args) = command.command_and_args();
    let mut process = Command::new(cue_bin);
    process
        .current_dir(workspace.target_dir())
        .arg(subcommand)
        .arg(format!(".:{}", workspace.module_package()))
        .arg("-e")
        .arg(export_expression(workspace, env, backend_override_value))
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    process
}

fn terraform_export_command(
    cue_bin: &Path,
    workspace: &Workspace,
    env: &Env,
    backend_override_value: &str,
    output_file: &Path,
) -> Result<Command> {
    let mut process = Command::new(cue_bin);
    process
        .current_dir(workspace.target_dir())
        .arg("export")
        .arg(format!(".:{}", workspace.module_package()))
        .arg("-e")
        .arg(export_expression(workspace, env, backend_override_value))
        .arg("-f")
        .arg("-o")
        .arg(
            output_file
                .strip_prefix(workspace.target_dir())
                .into_diagnostic()?,
        )
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    Ok(process)
}

fn terraform_command(tf_bin: &Path, output_dir: &Path, args: &[String]) -> Command {
    let mut process = Command::new(tf_bin);
    process
        .current_dir(output_dir)
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    process
}

pub fn run_cue(
    logger: &Logger,
    workspace: &Workspace,
    env: &Env,
    cue_bin: &Path,
    backend_override_value: &str,
    command: &CueCommand,
) -> Result<ExitStatus> {
    let (subcommand, _) = command.command_and_args();
    debug!(
        logger,
        "Executing cue {} with expression: {}",
        subcommand,
        export_expression(workspace, env, backend_override_value)
    );
    run_command(
        logger,
        &mut cue_command(cue_bin, workspace, env, backend_override_value, command),
    )
}

pub fn run_tf(
    logger: &Logger,
    workspace: &Workspace,
    env: &Env,
    cue_bin: &Path,
    tf_bin: &Path,
    backend_override_value: &str,
    args: &[String],
) -> Result<ExitStatus> {
    let output_dir = workspace.target_dir().join(OUTPUT_FOLDER_NAME).join(env);
    std::fs::create_dir_all(&output_dir)
        .into_diagnostic()
        .map_err(|error| miette::miette!("Failed to create output directory: {error}"))?;
    let output_file = output_dir.join(OUTPUT_FILE_NAME);

    let export_status = run_command(
        logger,
        &mut terraform_export_command(
            cue_bin,
            workspace,
            env,
            backend_override_value,
            &output_file,
        )?,
    )?;
    if !export_status.success() {
        return Ok(export_status);
    }

    run_command(logger, &mut terraform_command(tf_bin, &output_dir, args))
}

fn run_command(logger: &Logger, command: &mut Command) -> Result<ExitStatus> {
    let program = command.get_program().to_str().unwrap();
    let args = command.get_args().map(|arg| arg.to_str().unwrap());
    let command_string = shell_words::join(std::iter::once(program).chain(args));

    info!(
        logger,
        "From: {}\n   Running: {}",
        command
            .get_current_dir()
            .map_or(Cow::from("<None>"), Path::to_string_lossy),
        command_string
    );
    command.status().into_diagnostic()
}

pub fn resolve_tool(path: &Path) -> Result<PathBuf> {
    which::which(path)
        .into_diagnostic()
        .map_err(|error| miette::miette!("Could not find tool '{path:?}': {error}"))
}

#[cfg(test)]
mod tests {
    use super::{cue_command, run_tf};
    use crate::cli::{CueCommand, ModuleTarget};
    use crate::logger::Logger;
    use crate::test_support::TestDirectory;
    use crate::workspace::Workspace;
    use miette::{IntoDiagnostic, Result};
    use std::ffi::OsStr;
    use std::fs::{self, File};
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};

    fn write_executable(path: &Path, body: &str) -> Result<()> {
        let mut file = File::create(path).into_diagnostic()?;
        file.write_all(body.as_bytes()).into_diagnostic()?;
        file.sync_all().into_diagnostic()?;
        drop(file);
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).into_diagnostic()
    }

    fn test_workspace(temp: &TestDirectory) -> Result<Workspace> {
        let root = temp.path().join("workspace");
        let module = root.join("infra/neon");
        fs::create_dir_all(&module).into_diagnostic()?;
        fs::write(root.join(".cuetroot.cue"), "").into_diagnostic()?;
        Workspace::resolve(&module, None, &ModuleTarget::Relative(PathBuf::from(".")))
    }

    #[test]
    fn test_cue_command_uses_module_metadata() -> Result<()> {
        let temp = TestDirectory::new()?;
        let workspace = test_workspace(&temp)?;
        let env = "dev".to_owned();
        let command = cue_command(
            Path::new("cue"),
            &workspace,
            &env,
            "null",
            &CueCommand::Export {
                args: vec!["--out".to_owned(), "yaml".to_owned()],
            },
        );

        assert_eq!(command.get_program(), OsStr::new("cue"));
        assert_eq!(
            command.get_current_dir(),
            Some(temp.path().join("workspace/infra/neon").as_path())
        );
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            [
                "export",
                ".:neon",
                "-e",
                r#"((infra & { #metadata: { module: "infra/neon", localBackendOverride: null } }).out)["dev"]"#,
                "--out",
                "yaml",
            ]
            .map(OsStr::new)
        );
        Ok(())
    }

    #[test]
    fn test_terraform_export_failure_skips_terraform() -> Result<()> {
        let temp = TestDirectory::new()?;
        let cue_bin = temp.path().join("cue");
        let tf_bin = temp.path().join("tofu");
        let tf_marker = temp.path().join("tofu-ran");
        write_executable(&cue_bin, "#!/bin/sh\nexit 23\n")?;
        write_executable(
            &tf_bin,
            &format!("#!/bin/sh\ntouch '{}'\n", tf_marker.display()),
        )?;
        let workspace = test_workspace(&temp)?;
        let env = "dev".to_owned();

        let status = run_tf(
            &Logger::new(false),
            &workspace,
            &env,
            &cue_bin,
            &tf_bin,
            "null",
            &["plan".to_owned()],
        )?;

        assert_eq!(status.code(), Some(23));
        assert!(!tf_marker.exists());
        Ok(())
    }

    #[test]
    fn test_terraform_status_is_returned() -> Result<()> {
        let temp = TestDirectory::new()?;
        let cue_bin = temp.path().join("cue");
        let tf_bin = temp.path().join("tofu");
        write_executable(&cue_bin, "#!/bin/sh\nexit 0\n")?;
        write_executable(&tf_bin, "#!/bin/sh\nexit 19\n")?;
        let workspace = test_workspace(&temp)?;
        let env = "dev".to_owned();

        let status = run_tf(
            &Logger::new(false),
            &workspace,
            &env,
            &cue_bin,
            &tf_bin,
            "null",
            &["plan".to_owned()],
        )?;

        assert_eq!(status.code(), Some(19));
        Ok(())
    }
}

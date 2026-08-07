use crate::cli::{Env, validate_env};
use crate::workspace::Workspace;
use inquire::Select;
use miette::{IntoDiagnostic, Result};
use std::collections::BTreeSet;
use std::io::{IsTerminal, stderr, stdin};
use std::path::Path;
use std::process::{Command, Stdio};

pub fn discover(
    cue_bin: &Path,
    workspace: &Workspace,
    backend_override_value: &str,
) -> Result<Env> {
    let environments = populated(cue_bin, workspace, backend_override_value)?;

    select_with(
        environments,
        stdin().is_terminal() && stderr().is_terminal(),
        |items| {
            Select::new("Select an environment", items)
                .prompt_skippable()
                .into_diagnostic()
        },
    )
}

pub fn select<I, T>(environments: I) -> Result<T>
where
    I: IntoIterator<Item = T>,
    T: Ord + std::fmt::Display,
{
    select_with(
        environments,
        stdin().is_terminal() && stderr().is_terminal(),
        |items| {
            Select::new("Select an environment", items)
                .prompt_skippable()
                .into_diagnostic()
        },
    )
}

/// Returns every populated environment in a module.
pub fn populated(
    cue_bin: &Path,
    workspace: &Workspace,
    backend_override_value: &str,
) -> Result<BTreeSet<Env>> {
    let output = command(cue_bin, workspace, backend_override_value)
        .output()
        .into_diagnostic()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(miette::miette!(
            "Failed to discover populated environments ({}): {}",
            output.status,
            stderr.trim()
        ));
    }

    let environments: Vec<String> = serde_json::from_slice(&output.stdout)
        .into_diagnostic()
        .map_err(|error| miette::miette!("CUE returned invalid environment data: {error}"))?;
    environments
        .into_iter()
        .map(|environment| {
            validate_env(&environment).map_err(|error| {
                miette::miette!("CUE returned invalid environment '{environment}': {error}")
            })?;
            Ok(environment)
        })
        .collect()
}

fn command(cue_bin: &Path, workspace: &Workspace, backend_override_value: &str) -> Command {
    let module = serde_json::to_string(workspace.module_name())
        .expect("module name should always serialize");
    let expression = format!(
        "[for env, _ in (infra & {{ #metadata: {{ module: {module}, localBackendOverride: {backend_override_value} }} }})[\"in\"] {{env}}]"
    );
    let mut process = Command::new(cue_bin);
    process
        .current_dir(workspace.target_dir())
        .arg("export")
        .arg(format!(".:{}", workspace.module_package()))
        .arg("-e")
        .arg(expression)
        .arg("--out")
        .arg("json")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    process
}

fn select_with<F, I, T>(environments: I, interactive: bool, prompt: F) -> Result<T>
where
    F: FnOnce(Vec<T>) -> Result<Option<T>>,
    I: IntoIterator<Item = T>,
    T: Ord + std::fmt::Display,
{
    let mut environments: Vec<T> = environments.into_iter().collect();
    environments.sort();
    if environments.is_empty() {
        return Err(miette::miette!(
            "No populated environments exist in this module"
        ));
    }
    if environments.len() == 1 {
        return Ok(environments.pop().expect("one environment should exist"));
    }
    if !interactive {
        let environments = environments
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        return Err(miette::miette!(
            "Multiple populated environments exist ({}); select one explicitly with '-t :ENV'",
            environments
        ));
    }
    prompt(environments)?.ok_or_else(|| miette::miette!("Environment selection cancelled"))
}

#[cfg(test)]
mod tests {
    use super::{command, discover, populated, select_with};
    use crate::cli::ModuleTarget;
    use crate::test_support::TestDirectory;
    use crate::workspace::Workspace;
    use miette::{IntoDiagnostic, Result};
    use std::ffi::OsStr;
    use std::fs::{self, File};
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};

    fn test_workspace(temp: &TestDirectory) -> Result<Workspace> {
        let root = temp.path().join("workspace");
        let module = root.join("infra/neon");
        fs::create_dir_all(&module).into_diagnostic()?;
        fs::write(root.join(".cuetroot.cue"), "").into_diagnostic()?;
        Workspace::resolve(&module, None, &ModuleTarget::Relative(PathBuf::from(".")))
    }

    fn write_executable(path: &Path, body: &str) -> Result<()> {
        let mut file = File::create(path).into_diagnostic()?;
        file.write_all(body.as_bytes()).into_diagnostic()?;
        file.sync_all().into_diagnostic()?;
        drop(file);
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).into_diagnostic()
    }

    #[test]
    fn test_environment_command_queries_populated_input() -> Result<()> {
        let temp = TestDirectory::new()?;
        let workspace = test_workspace(&temp)?;

        let command = command(Path::new("cue"), &workspace, "null");

        assert_eq!(command.get_program(), OsStr::new("cue"));
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            [
                "export",
                ".:neon",
                "-e",
                r#"[for env, _ in (infra & { #metadata: { module: "infra/neon", localBackendOverride: null } })["in"] {env}]"#,
                "--out",
                "json",
            ]
            .map(OsStr::new)
        );
        Ok(())
    }

    #[test]
    fn test_populated_returns_set() -> Result<()> {
        let temp = TestDirectory::new()?;
        let cue_bin = temp.path().join("cue");
        write_executable(
            &cue_bin,
            "#!/usr/bin/env bash\nset -euo pipefail\nprintf '[\"prod\",\"dev\",\"dev\"]'\n",
        )?;
        let workspace = test_workspace(&temp)?;

        let environments = populated(&cue_bin, &workspace, "null")?;

        assert_eq!(
            environments,
            ["dev".to_owned(), "prod".to_owned()].into_iter().collect()
        );
        Ok(())
    }

    #[test]
    fn test_discover_reports_cue_failure() -> Result<()> {
        let temp = TestDirectory::new()?;
        let cue_bin = temp.path().join("cue");
        write_executable(
            &cue_bin,
            "#!/usr/bin/env bash\nset -euo pipefail\nprintf 'invalid module' >&2\nexit 17\n",
        )?;
        let workspace = test_workspace(&temp)?;

        let error = discover(&cue_bin, &workspace, "null")
            .expect_err("failed CUE query should be reported");

        assert!(error.to_string().contains("invalid module"));
        assert!(error.to_string().contains("exit status: 17"));
        Ok(())
    }

    #[test]
    fn test_populated_rejects_invalid_environment_name() -> Result<()> {
        let temp = TestDirectory::new()?;
        let cue_bin = temp.path().join("cue");
        write_executable(
            &cue_bin,
            "#!/usr/bin/env bash\nset -euo pipefail\nprintf '[\"invalid/name\"]'\n",
        )?;
        let workspace = test_workspace(&temp)?;

        let error = populated(&cue_bin, &workspace, "null")
            .expect_err("invalid environment should be rejected");

        assert!(error.to_string().contains("invalid/name"));
        assert!(error.to_string().contains("[A-Za-z0-9_-]+"));
        Ok(())
    }

    #[test]
    fn test_select_environment_rejects_empty_list() {
        let error = select_with::<_, _, &str>([], false, |_| unreachable!())
            .expect_err("empty environment list should fail");

        assert!(error.to_string().contains("No populated environments"));
    }

    #[test]
    fn test_select_environment_uses_only_environment() -> Result<()> {
        let environment = select_with(["dev"], false, |_| unreachable!())?;

        assert_eq!(environment, "dev");
        Ok(())
    }

    #[test]
    fn test_select_environment_rejects_multiple_without_terminal() {
        let error = select_with(["dev", "prod"], false, |_| unreachable!())
            .expect_err("non-interactive selection should fail");

        assert!(error.to_string().contains("dev, prod"));
        assert!(error.to_string().contains("-t :ENV"));
    }

    #[test]
    fn test_select_environment_prompts_for_multiple() -> Result<()> {
        let environment = select_with(["prod", "dev"], true, |options| {
            assert_eq!(options, ["dev", "prod"]);
            Ok(Some(options[1]))
        })?;

        assert_eq!(environment, "prod");
        Ok(())
    }

    #[test]
    fn test_select_environment_reports_cancellation() {
        let error = select_with(["dev", "prod"], true, |_| Ok(None))
            .expect_err("cancelled selection should fail");

        assert!(error.to_string().contains("selection cancelled"));
    }
}

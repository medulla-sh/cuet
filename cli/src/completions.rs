use crate::cli::{Cli, ModuleTarget};
use crate::environment;
use crate::workspace::{Workspace, discover_modules, resolve_root};
use clap::CommandFactory;
use clap_complete::CompleteEnv;
use clap_complete::Shell;
use clap_complete::engine::{CompletionCandidate, complete as complete_args};
use clap_complete::env::{Bash, Elvish, EnvCompleter, Fish, Powershell, Shells};
use miette::{IntoDiagnostic, Result};
use std::ffi::{OsStr, OsString};
use std::io::Write;
use std::path::Path;
use std::process::Command;

const MODULE_TAG: &str = "cuet-target-module";
const MODULE_MARKER: &str = "__cuet_target_module__";

pub fn complete() {
    let shells: [&dyn EnvCompleter; 5] = [&Bash, &Elvish, &Fish, &Powershell, &CuetZsh];
    CompleteEnv::with_factory(Cli::command)
        .shells(Shells(&shells))
        .complete();
}

pub fn write_registration(shell: Shell, output: &mut impl Write) -> Result<()> {
    let executable = std::env::current_exe().into_diagnostic()?;
    let registration = Command::new(executable)
        .env("COMPLETE", shell.to_string())
        .output()
        .into_diagnostic()?;
    if !registration.status.success() {
        return Err(miette::miette!(
            "Failed to generate {shell} completions ({}): {}",
            registration.status,
            String::from_utf8_lossy(&registration.stderr).trim()
        ));
    }

    output.write_all(&registration.stdout).into_diagnostic()
}

pub fn target_candidates(current: &OsStr) -> Vec<CompletionCandidate> {
    let Some(current) = current.to_str() else {
        return Vec::new();
    };
    let Ok(current_dir) = std::env::current_dir() else {
        return Vec::new();
    };

    let completing_module = !current.contains(':');
    target_values(current, &current_dir, Path::new("cue"))
        .unwrap_or_default()
        .into_iter()
        .map(|value| {
            let candidate = CompletionCandidate::new(value);
            if completing_module {
                candidate.tag(Some(MODULE_TAG.into()))
            } else {
                candidate
            }
        })
        .collect()
}

fn target_values(current: &str, current_dir: &Path, cue_bin: &Path) -> Result<Vec<String>> {
    let Some((module, environment_prefix)) = current.split_once(':') else {
        let root = resolve_root(current_dir, None)?;
        return Ok(discover_modules(&root)?
            .into_iter()
            .map(|module| {
                if module == "." {
                    module
                } else {
                    format!("/{module}")
                }
            })
            .filter(|candidate| module_candidate_matches(candidate, current))
            .collect());
    };

    let module_target = ModuleTarget::from_cli_component(module);
    let workspace = Workspace::resolve(current_dir, None, &module_target)?;
    let mut environments: Vec<_> = environment::populated(cue_bin, &workspace, "null")?
        .into_iter()
        .filter(|environment| environment.starts_with(environment_prefix))
        .map(|environment| format!("{module}:{environment}"))
        .collect();
    environments.sort();
    Ok(environments)
}

// clap_complete cannot represent a removable suffix, so preserve module tags in
// the Zsh protocol and let Zsh insert `:` separately from the candidate value.
struct CuetZsh;

impl EnvCompleter for CuetZsh {
    fn name(&self) -> &'static str {
        "zsh"
    }

    fn is(&self, name: &str) -> bool {
        name == "zsh"
    }

    fn write_registration(
        &self,
        var: &str,
        name: &str,
        bin: &str,
        completer: &str,
        output: &mut dyn Write,
    ) -> std::io::Result<()> {
        let name = name.replace('-', "_");
        let bin = shell_quote(bin);
        let completer = shell_quote(completer);
        let script = r#"#compdef @@BIN@@
function _clap_dynamic_completer_@@NAME@@() {
    local _CLAP_COMPLETE_INDEX=$(expr $CURRENT - 1)
    local _CLAP_IFS=$'\n'

    local completions=("${(@f)$( \
        _CLAP_IFS="$_CLAP_IFS" \
        _CLAP_COMPLETE_INDEX="$_CLAP_COMPLETE_INDEX" \
        @@VAR@@="zsh" \
        @@COMPLETER@@ -- "${words[@]}" 2>/dev/null \
    )}")

    if [[ -n $completions ]]; then
        local -a dirs=()
        local -a modules=()
        local -a other=()
        local completion
        for completion in $completions; do
            if [[ "$completion" == @@MODULE_MARKER@@$'\t'* ]]; then
                modules+=("${completion#*$'\t'}")
                continue
            fi

            local value="${completion%%:*}"
            if [[ "$value" == */ ]]; then
                local dir_no_slash="${value%/}"
                if [[ "$completion" == *:* ]]; then
                    local desc="${completion#*:}"
                    dirs+=("$dir_no_slash:$desc")
                else
                    dirs+=("$dir_no_slash")
                fi
            else
                other+=("$completion")
            fi
        done
        [[ -n $dirs ]] && _describe -V 'values' dirs -S '/' -r '/'
        [[ -n $modules ]] && _describe -V 'modules' modules -S ':' -r ' '
        [[ -n $other ]] && _describe -V 'values' other
    fi
}

compdef _clap_dynamic_completer_@@NAME@@ @@BIN@@"#;
        write_template(
            output,
            script,
            &[
                ("BIN", bin.as_str()),
                ("NAME", name.as_str()),
                ("COMPLETER", completer.as_str()),
                ("MODULE_MARKER", MODULE_MARKER),
                ("VAR", var),
            ],
        )?;
        writeln!(output)
    }

    fn write_complete(
        &self,
        command: &mut clap::Command,
        mut args: Vec<OsString>,
        current_dir: Option<&Path>,
        output: &mut dyn Write,
    ) -> std::io::Result<()> {
        let index = std::env::var("_CLAP_COMPLETE_INDEX")
            .ok()
            .and_then(|index| index.parse().ok())
            .unwrap_or_default();
        let separator = std::env::var("_CLAP_IFS").unwrap_or_else(|_| "\n".to_owned());
        if args.len() == index {
            args.push(OsString::new());
        }

        let candidates = complete_args(command, args, index, current_dir)?;
        let module_tag = MODULE_TAG.into();
        for (index, candidate) in candidates.iter().enumerate() {
            if index != 0 {
                write!(output, "{separator}")?;
            }
            if candidate.get_tag().is_some_and(|tag| tag == &module_tag) {
                write!(
                    output,
                    "{MODULE_MARKER}\t{}",
                    candidate.get_value().to_string_lossy()
                )?;
                continue;
            }

            write!(
                output,
                "{}",
                escape_zsh_value(&candidate.get_value().to_string_lossy())
            )?;
            if let Some(help) = candidate.get_help() {
                write!(
                    output,
                    ":{}",
                    help.to_string()
                        .lines()
                        .next()
                        .unwrap_or_default()
                        .replace('\\', "\\\\")
                )?;
            }
        }
        Ok(())
    }
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn write_template(
    output: &mut dyn Write,
    template: &str,
    replacements: &[(&str, &str)],
) -> std::io::Result<()> {
    let mut parts = template.split("@@");
    if let Some(text) = parts.next() {
        output.write_all(text.as_bytes())?;
    }
    while let Some(key) = parts.next() {
        let value = replacements
            .iter()
            .find_map(|(candidate, value)| (*candidate == key).then_some(*value))
            .ok_or_else(|| std::io::Error::other(format!("Unknown template key '{key}'")))?;
        output.write_all(value.as_bytes())?;
        if let Some(text) = parts.next() {
            output.write_all(text.as_bytes())?;
        }
    }
    Ok(())
}

fn escape_zsh_value(value: &str) -> String {
    value.replace('\\', "\\\\").replace(':', "\\:")
}

fn module_candidate_matches(candidate: &str, current: &str) -> bool {
    candidate.starts_with(current)
        || (!current.starts_with('/')
            && candidate
                .strip_prefix('/')
                .is_some_and(|candidate| candidate.starts_with(current)))
}

#[cfg(test)]
mod tests {
    use super::target_values;
    use crate::test_support::TestDirectory;
    use miette::{IntoDiagnostic, Result};
    use std::ffi::OsStr;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn test_target_values_complete_workspace_modules_without_cue() -> Result<()> {
        let temp = TestDirectory::new()?;
        let root = temp.path().join("workspace");
        fs::create_dir_all(root.join("infra/neon")).into_diagnostic()?;
        fs::create_dir_all(root.join("services/api")).into_diagnostic()?;
        fs::write(root.join(".cuetroot.cue"), "").into_diagnostic()?;
        fs::write(root.join("cuet.cue"), "").into_diagnostic()?;
        fs::write(root.join("infra/neon/cuet.cue"), "").into_diagnostic()?;
        fs::write(root.join("services/api/cuet.cue"), "").into_diagnostic()?;

        let values = target_values("", &root, &temp.path().join("missing-cue"))?;

        assert_eq!(values, [".", "/infra/neon", "/services/api"]);
        assert_eq!(
            target_values("infra/n", &root, &temp.path().join("missing-cue"))?,
            ["/infra/neon"]
        );
        Ok(())
    }

    #[test]
    fn test_target_values_complete_selected_module_environments() -> Result<()> {
        let temp = TestDirectory::new()?;
        let root = temp.path().join("workspace");
        let module = root.join("infra/neon");
        fs::create_dir_all(&module).into_diagnostic()?;
        fs::write(root.join(".cuetroot.cue"), "").into_diagnostic()?;
        fs::write(module.join("cuet.cue"), "").into_diagnostic()?;
        let cue = temp.path().join("cue");
        let pending_cue = temp.path().join("cue.tmp");
        fs::write(
            &pending_cue,
            b"#!/usr/bin/env bash\nprintf '[\"prod\",\"dev\",\"stage\"]'\n",
        )
        .into_diagnostic()?;
        fs::set_permissions(&pending_cue, fs::Permissions::from_mode(0o755)).into_diagnostic()?;
        fs::rename(pending_cue, &cue).into_diagnostic()?;

        let values = target_values("/infra/neon:d", &root, &cue)?;

        assert_eq!(values, ["/infra/neon:dev"]);
        Ok(())
    }

    #[test]
    fn test_target_values_ignore_discovery_errors_at_completion_boundary() {
        let values = super::target_candidates(OsStr::new(""));

        assert!(values.is_empty());
    }
}

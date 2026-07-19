use crate::cli::ModuleTarget;
use miette::{IntoDiagnostic, Result};
use std::path::{Path, PathBuf};

const WORKSPACE_MARKER: &str = ".cuetroot.cue";

pub struct Workspace {
    root: PathBuf,
    target_dir: PathBuf,
    module_name: String,
    module_package: String,
}

impl Workspace {
    pub fn resolve(
        current_dir: &Path,
        root_override: Option<&Path>,
        module: &ModuleTarget,
    ) -> Result<Self> {
        let current_dir = canonicalize_directory(current_dir, "Current directory")?;
        let root = match root_override {
            Some(path) => resolve_explicit_root(&current_dir, path)?,
            None => find_root(&current_dir)?,
        };
        let target = match module {
            ModuleTarget::Relative(path) => {
                canonicalize_directory(&current_dir.join(path), "Target module")?
            }
            ModuleTarget::WorkspaceRelative(path) => {
                canonicalize_directory(&root.join(path), "Target module")?
            }
        };
        let module_path = target.strip_prefix(&root).map_err(|_| {
            miette::miette!(
                "Target module '{}' must be inside workspace '{}'",
                target.display(),
                root.display()
            )
        })?;
        let module_name = module_path
            .to_str()
            .ok_or_else(|| miette::miette!("Target module path must be valid UTF-8"))?
            .to_owned();
        let module_package = target
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| miette::miette!("Could not infer CUE package from target module"))?
            .to_owned();

        Ok(Self {
            root,
            target_dir: target,
            module_name,
            module_package,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn target_dir(&self) -> &Path {
        &self.target_dir
    }

    pub fn module_name(&self) -> &str {
        &self.module_name
    }

    pub fn module_package(&self) -> &str {
        &self.module_package
    }
}

fn canonicalize_directory(path: &Path, label: &str) -> Result<PathBuf> {
    let path = path.canonicalize().into_diagnostic().map_err(|error| {
        miette::miette!(
            "{label} '{}' could not be resolved: {error}",
            path.display()
        )
    })?;
    if !path.is_dir() {
        return Err(miette::miette!(
            "{label} '{}' is not a directory",
            path.display()
        ));
    }
    Ok(path)
}

fn resolve_explicit_root(current_dir: &Path, path: &Path) -> Result<PathBuf> {
    let path = if path.is_absolute() {
        path.to_owned()
    } else {
        current_dir.join(path)
    };
    let root = canonicalize_directory(&path, "Workspace root")?;
    if !root.join(WORKSPACE_MARKER).is_file() {
        return Err(miette::miette!(
            "Workspace root '{}' does not contain {WORKSPACE_MARKER}",
            root.display()
        ));
    }
    Ok(root)
}

fn find_root(start_path: &Path) -> Result<PathBuf> {
    start_path
        .ancestors()
        .find(|path| path.join(WORKSPACE_MARKER).is_file())
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            miette::miette!(
                "Could not find {WORKSPACE_MARKER} from '{}'; pass -w to select a workspace",
                start_path.display()
            )
        })
}

#[cfg(test)]
mod tests {
    use super::{Workspace, find_root};
    use crate::cli::ModuleTarget;
    use crate::test_support::TestDirectory;
    use miette::{IntoDiagnostic, Result};
    use std::fs;
    use std::os::unix::fs::symlink;
    use std::path::{Path, PathBuf};

    fn create_workspace(temp: &TestDirectory) -> Result<std::path::PathBuf> {
        let root = temp.path().join("workspace");
        fs::create_dir(&root).into_diagnostic()?;
        fs::write(root.join(".cuetroot.cue"), "").into_diagnostic()?;
        Ok(root)
    }

    #[test]
    fn test_find_root_uses_nearest_workspace() -> Result<()> {
        let temp = TestDirectory::new()?;
        let workspace = create_workspace(&temp)?;
        let nested_workspace = workspace.join("nested");
        let module = nested_workspace.join("infra/module");
        fs::create_dir_all(&module).into_diagnostic()?;
        fs::write(nested_workspace.join(".cuetroot.cue"), "").into_diagnostic()?;

        let root = find_root(&module)?;

        assert_eq!(root, nested_workspace);
        Ok(())
    }

    #[test]
    fn test_workspace_resolves_relative_module_from_current_directory() -> Result<()> {
        let temp = TestDirectory::new()?;
        let root = create_workspace(&temp)?;
        let current = root.join("infra");
        let module = current.join("neon");
        fs::create_dir_all(&module).into_diagnostic()?;

        let workspace = Workspace::resolve(
            &current,
            None,
            &ModuleTarget::Relative(Path::new("neon").to_owned()),
        )?;

        assert_eq!(workspace.root(), root);
        assert_eq!(workspace.target_dir(), module);
        assert_eq!(workspace.module_name(), "infra/neon");
        assert_eq!(workspace.module_package(), "neon");
        Ok(())
    }

    #[test]
    fn test_workspace_resolves_leading_slash_from_workspace_root() -> Result<()> {
        let temp = TestDirectory::new()?;
        let root = create_workspace(&temp)?;
        let current = root.join("other");
        let module = root.join("infra/neon");
        fs::create_dir_all(&current).into_diagnostic()?;
        fs::create_dir_all(&module).into_diagnostic()?;

        let workspace = Workspace::resolve(
            &current,
            None,
            &ModuleTarget::WorkspaceRelative(Path::new("infra/neon").to_owned()),
        )?;

        assert_eq!(workspace.target_dir(), module);
        assert_eq!(workspace.module_name(), "infra/neon");
        Ok(())
    }

    #[test]
    fn test_workspace_uses_exact_explicit_root() -> Result<()> {
        let temp = TestDirectory::new()?;
        let root = create_workspace(&temp)?;
        let current = root.join("infra/neon");
        fs::create_dir_all(&current).into_diagnostic()?;

        let workspace = Workspace::resolve(
            &current,
            Some(&root),
            &ModuleTarget::Relative(PathBuf::from(".")),
        )?;

        assert_eq!(workspace.root(), root);
        assert_eq!(workspace.target_dir(), current);
        Ok(())
    }

    #[test]
    fn test_workspace_rejects_unmarked_explicit_root() -> Result<()> {
        let temp = TestDirectory::new()?;
        let root = create_workspace(&temp)?;
        let current = root.join("infra/neon");
        fs::create_dir_all(&current).into_diagnostic()?;

        let error = Workspace::resolve(
            &current,
            Some(&current),
            &ModuleTarget::Relative(PathBuf::from(".")),
        )
        .err()
        .expect("unmarked root should fail");

        assert!(error.to_string().contains("does not contain .cuetroot.cue"));
        Ok(())
    }

    #[test]
    fn test_workspace_allows_parent_traversal_inside_workspace() -> Result<()> {
        let temp = TestDirectory::new()?;
        let root = create_workspace(&temp)?;
        let current = root.join("infra/first");
        let sibling = root.join("infra/second");
        fs::create_dir_all(&current).into_diagnostic()?;
        fs::create_dir_all(&sibling).into_diagnostic()?;

        let workspace = Workspace::resolve(
            &current,
            None,
            &ModuleTarget::Relative(Path::new("../second").to_owned()),
        )?;

        assert_eq!(workspace.target_dir(), sibling);
        Ok(())
    }

    #[test]
    fn test_workspace_rejects_module_outside_workspace() -> Result<()> {
        let temp = TestDirectory::new()?;
        let root = create_workspace(&temp)?;
        let current = root.join("infra");
        let outside = temp.path().join("outside");
        fs::create_dir(&current).into_diagnostic()?;
        fs::create_dir(&outside).into_diagnostic()?;

        let error = Workspace::resolve(
            &current,
            None,
            &ModuleTarget::Relative(Path::new("../../outside").to_owned()),
        )
        .err()
        .expect("workspace escape should fail");

        assert!(error.to_string().contains("must be inside workspace"));
        Ok(())
    }

    #[test]
    fn test_workspace_rejects_symlink_outside_workspace() -> Result<()> {
        let temp = TestDirectory::new()?;
        let root = create_workspace(&temp)?;
        let current = root.join("infra");
        let outside = temp.path().join("outside");
        fs::create_dir(&current).into_diagnostic()?;
        fs::create_dir(&outside).into_diagnostic()?;
        symlink(&outside, current.join("linked")).into_diagnostic()?;

        let error = Workspace::resolve(
            &current,
            None,
            &ModuleTarget::Relative(Path::new("linked").to_owned()),
        )
        .err()
        .expect("symlink escape should fail");

        assert!(error.to_string().contains("must be inside workspace"));
        Ok(())
    }
}

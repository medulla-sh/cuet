use miette::{IntoDiagnostic, Result};
use std::path::{Path, PathBuf};

pub struct Workspace {
    root: PathBuf,
    target_dir: PathBuf,
    module_name: String,
    module_package: String,
}

impl Workspace {
    pub fn resolve(path: Option<PathBuf>) -> Result<Self> {
        let target_dir = match path {
            Some(path) => path,
            None => std::env::current_dir().into_diagnostic()?,
        }
        .canonicalize()
        .into_diagnostic()?;
        let root = find_root(&target_dir)?;
        let module_name = target_dir
            .strip_prefix(&root)
            .into_diagnostic()
            .map_err(|_| miette::miette!("Target file must be inside the cuet workspace"))?
            .to_string_lossy()
            .into_owned();
        let module_package = target_dir
            .file_name()
            .ok_or_else(|| miette::miette!("Could not infer module package from path"))?
            .to_string_lossy()
            .into_owned();

        Ok(Self {
            root,
            target_dir,
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

fn find_root(start_path: &Path) -> Result<PathBuf> {
    start_path
        .ancestors()
        .find(|path| path.join(".cuetroot.cue").exists())
        .map(Path::to_path_buf)
        .ok_or_else(|| miette::miette!("Could not find .cuetroot.cue in ancestors"))
}

#[cfg(test)]
mod tests {
    use super::{Workspace, find_root};
    use crate::test_support::TestDirectory;
    use miette::{IntoDiagnostic, Result};
    use std::fs;

    #[test]
    fn test_find_root_uses_nearest_workspace() -> Result<()> {
        let temp = TestDirectory::new()?;
        let workspace = temp.path().join("workspace");
        let nested_workspace = workspace.join("nested");
        let module = nested_workspace.join("infra/module");
        fs::create_dir_all(&module).into_diagnostic()?;
        fs::write(workspace.join(".cuetroot.cue"), "").into_diagnostic()?;
        fs::write(nested_workspace.join(".cuetroot.cue"), "").into_diagnostic()?;

        let root = find_root(&module)?;

        assert_eq!(root, nested_workspace);
        Ok(())
    }

    #[test]
    fn test_workspace_resolves_module_metadata() -> Result<()> {
        let temp = TestDirectory::new()?;
        let workspace_root = temp.path().join("workspace");
        let module = workspace_root.join("infra/neon");
        fs::create_dir_all(&module).into_diagnostic()?;
        fs::write(workspace_root.join(".cuetroot.cue"), "").into_diagnostic()?;

        let workspace = Workspace::resolve(Some(module.clone()))?;

        assert_eq!(workspace.root(), workspace_root);
        assert_eq!(workspace.target_dir(), module);
        assert_eq!(workspace.module_name(), "infra/neon");
        assert_eq!(workspace.module_package(), "neon");
        Ok(())
    }
}

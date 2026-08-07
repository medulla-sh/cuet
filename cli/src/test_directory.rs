use crate::cli::ModuleTarget;
use crate::workspace::Workspace;
use miette::{IntoDiagnostic, Result};
use std::fs::{self, File};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

pub struct TestDirectory(tempfile::TempDir);

impl TestDirectory {
    pub fn new() -> Result<Self> {
        tempfile::tempdir().map(Self).into_diagnostic()
    }

    pub fn path(&self) -> &Path {
        self.0.path()
    }

    pub fn workspace(&self) -> Result<Workspace> {
        let root = self.path().join("workspace");
        let module = root.join("infra/neon");
        fs::create_dir_all(&module).into_diagnostic()?;
        fs::write(root.join(".cuetroot.cue"), "").into_diagnostic()?;
        Workspace::resolve(&module, None, &ModuleTarget::Relative(PathBuf::from(".")))
    }

    pub fn write_executable(&self, path: &Path, body: &str) -> Result<()> {
        if !path.starts_with(self.path()) {
            return Err(miette::miette!(
                "Test executable '{}' must be inside '{}'",
                path.display(),
                self.path().display()
            ));
        }
        let pending = path.with_extension("tmp");
        let mut file = File::create(&pending).into_diagnostic()?;
        file.write_all(body.as_bytes()).into_diagnostic()?;
        file.sync_all().into_diagnostic()?;
        drop(file);
        fs::set_permissions(&pending, fs::Permissions::from_mode(0o755)).into_diagnostic()?;
        fs::rename(pending, path).into_diagnostic()?;
        // Overlay filesystems can briefly report ETXTBSY immediately after publishing an executable.
        std::thread::sleep(Duration::from_millis(10));
        Ok(())
    }
}

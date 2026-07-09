//! Simple wrapper for `dpm` to execute from build scripts.

use std::{
    env, error,
    ffi::OsStr,
    fmt,
    io::{self, Write as _},
    path::{Path, PathBuf},
    process::Command,
};

/// Build with default config
pub fn build() -> Result<BuildResult, DpmError> {
    let config = Config::default();
    config.build()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum MultiPackage {
    #[default]
    Yes,
    No,
    Auto,
}

impl fmt::Display for MultiPackage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            MultiPackage::Yes => "yes",
            MultiPackage::No => "no",
            MultiPackage::Auto => "auto",
        })
    }
}

#[derive(Clone, Debug)]
pub struct BuildResult {
    pub output: PathBuf,
}

struct ResolvedConfig {
    package_root: PathBuf,
    output: PathBuf,
    multi_package: MultiPackage,
    dpm_exe: PathBuf,
}

impl ResolvedConfig {
    fn build(&self) -> Result<BuildResult, DpmError> {
        let mut cmd = Command::new(&self.dpm_exe);
        cmd.env("DAML_PACKAGE", &self.package_root)
            .arg("build")
            .args(["--enable-multi-package", &self.multi_package.to_string()])
            .args([OsStr::new("--output"), self.output.as_os_str()]);

        let output = cmd.output().map_err(DpmError::DpmExecutionFailed)?;

        if !output.status.success() {
            eprintln!("---- dpm stdout");
            io::stderr()
                .write_all(output.stdout.as_slice())
                .map_err(DpmError::FailedToWriteOutput)?;

            eprintln!("---- dpm stderr");
            io::stderr()
                .write_all(output.stderr.as_slice())
                .map_err(DpmError::FailedToWriteOutput)?;

            return Err(DpmError::BuildFailed);
        }

        Ok(BuildResult {
            output: self.output.clone(),
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct Config {
    package_root: Option<PathBuf>,
    output: Option<PathBuf>,
    multi_package: MultiPackage,
    dpm_exe: Option<PathBuf>,
}

const DEFAULT_PACKAGE_NAME: &str = "package.dar";

impl Config {
    pub fn package_root(&mut self, path: impl AsRef<Path>) -> &mut Self {
        self.package_root = Some(path.as_ref().to_path_buf());
        self
    }

    pub fn output(&mut self, path: impl AsRef<Path>) -> &mut Self {
        self.output = Some(path.as_ref().to_path_buf());
        self
    }

    pub fn multi_package(&mut self, multi_package: MultiPackage) -> &mut Self {
        self.multi_package = multi_package;
        self
    }

    pub fn disable_multi_package(&mut self) -> &mut Self {
        self.multi_package = MultiPackage::No;
        self
    }

    pub fn dpm_exe(&mut self, path: impl AsRef<Path>) -> &mut Self {
        self.dpm_exe = Some(path.as_ref().to_path_buf());
        self
    }

    fn resolve(&self) -> Result<ResolvedConfig, DpmError> {
        let dpm_exe = if let Ok(dpm_exe) = env::var("DPM") {
            PathBuf::from(dpm_exe)
        } else if let Some(dpm_exe) = &self.dpm_exe {
            dpm_exe.clone()
        } else {
            PathBuf::from("dpm")
        };

        let package_root = if let Some(package_root) = &self.package_root {
            package_root.clone()
        } else {
            PathBuf::from(env::var("CARGO_MANIFEST_DIR").map_err(|_| DpmError::PackageRootNotSet)?)
        };

        let output = if let Some(output) = &self.output {
            output.clone()
        } else if let Ok(out_dir) = env::var("OUT_DIR") {
            PathBuf::from(out_dir).join(DEFAULT_PACKAGE_NAME)
        } else {
            return Err(DpmError::OutDirNotSet);
        };

        Ok(ResolvedConfig {
            package_root,
            output,
            multi_package: self.multi_package,
            dpm_exe,
        })
    }

    pub fn build(&self) -> Result<BuildResult, DpmError> {
        let resolved = self.resolve()?;
        resolved.build()
    }
}

#[derive(Debug)]
pub enum DpmError {
    DpmExecutionFailed(io::Error),
    FailedToWriteOutput(io::Error),
    BuildFailed,
    OutDirNotSet,
    PackageRootNotSet,
}

impl fmt::Display for DpmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            DpmError::DpmExecutionFailed(_) => "dpm execution failed",
            DpmError::FailedToWriteOutput(_) => "failed to write output of dpm process",
            DpmError::BuildFailed => "dpm build failed",
            DpmError::OutDirNotSet => "output directory is not set (use config or OUT_DIR)",
            DpmError::PackageRootNotSet => "Daml package root not set",
        })
    }
}

impl error::Error for DpmError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            DpmError::DpmExecutionFailed(error) => Some(error),
            DpmError::FailedToWriteOutput(error) => Some(error),
            _ => None,
        }
    }
}

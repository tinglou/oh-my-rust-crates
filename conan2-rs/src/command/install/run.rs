//! helper to integrate conan with cargo
//! 1. generate `conan install` command line
//! 2. run and parse conan output
//! 3. visit `conan graph`
//!   * output cargo link options `cargo:rustc-link-search` and `cargo:rustc-link-lib` ('static' or 'dylib')
//!   * gather `include_dirs` for `bindgen` to generate rust bindings
//!

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::Command;

use thiserror::Error;

use super::InstallCommandArgs;
use crate::command::find_conan_program;
use crate::model::ConanRoot;

#[derive(Debug, Error)]
pub enum ConanInstallError {
    #[error("conan program not found")]
    ConanNotFound,

    #[error("exec error: {0}")]
    Exec(String),

    #[error("Output dir not specified")]
    NoOutput,

    #[error("Io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Json error: {0}")]
    Model(#[from] crate::model::ModelError),
}

pub struct ConanInstall {
    pub args: InstallCommandArgs,
}

impl InstallCommandArgs {
    pub fn into_command(self) -> ConanInstall {
        ConanInstall { args: self }
    }
}

impl ConanInstall {
    pub fn builder() -> Result<InstallCommandArgs, ConanInstallError> {
        let args = InstallCommandArgs::default()
            .output_folder(
                std::env::var_os("OUT_DIR")
                    .ok_or(ConanInstallError::NoOutput)?
                    .to_str()
                    .ok_or(ConanInstallError::NoOutput)?,
            )
            .format("json");

        Ok(args)
    }

    pub fn cmdline_args(&self) -> Result<Vec<String>, ConanInstallError> {
        let args = self.args.clone();

        let output_file = self.out_file()?;
        println!("cargo:warning={}", output_file.to_string_lossy());
        let args = args.out_file(output_file.to_string_lossy());

        Ok(args.to_args())
    }

    fn out_file(&self) -> Result<PathBuf, ConanInstallError> {
        let output_file = self
            .args
            .output_folder
            .clone()
            .ok_or(ConanInstallError::NoOutput)?;
        let mut path = PathBuf::from(&output_file);
        path.push(crate::CONAN_OUT_FILE);
        Ok(path)
    }

    fn parse_output<P: AsRef<Path>>(out_file: P) -> Result<ConanRoot, ConanInstallError> {
        Ok(serde_json::from_reader(BufReader::new(File::open(
            out_file,
        )?))?)
    }

    pub fn run(&self) -> Result<ConanRoot, ConanInstallError> {
        let args = self.cmdline_args()?;
        let _conan_file = self
            .args
            .path
            .as_ref()
            .ok_or(ConanInstallError::ConanNotFound)?;
        let out_file = self.out_file()?;
        // try to parse output file first
        if let Ok(root) = Self::parse_output(&out_file) {
            return Ok(root);
        }

        let program = find_conan_program().ok_or(ConanInstallError::ConanNotFound)?;
        println!("cargo:warning=cmdline: {} {:?}", program.display(), args);
        let mut command = Command::new(program);
        let status = command.args(args).status()?;
        if status.success() {
            //  parse json
            let root: ConanRoot = Self::parse_output(out_file)?;
            return Ok(root);
        }

        Err(ConanInstallError::Exec(format!(
            "conan install failed: {}",
            status
        )))
    }

    pub fn link_options(root: &ConanRoot, deps: &[&str]) -> Result<(), ConanInstallError> {
        let cpp_info = root.get_cpp_info(deps)?;
        for libdir in cpp_info.lib_dirs.iter() {
            println!("cargo:rustc-link-search={}", libdir);
        }
        for lib in cpp_info.libs.iter() {
            println!("cargo:rustc-link-lib=static={}", lib);
        }

        Ok(())
    }

    // pub fn build(&self, deps: &[&str]) -> Result<(), ConanInstallError> {
    //     let root = self.run()?;
    //     Self::link_options(&root, deps)?;
    //     Ok(())
    // }
}

#[cfg(test)]
mod tests {
    use anyhow::Context;

    use super::*;
    use crate::command::BuildPolicy;

    #[test]
    fn it_works() -> anyhow::Result<()> {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")?;
        let out_path = std::path::Path::new(&manifest_dir).join("target");
        let conan_file = std::path::Path::new(&manifest_dir).join("tests/conanfile.txt");

        unsafe { std::env::set_var("OUT_DIR", out_path.to_str().context("None")?) };

        let builder = ConanInstall::builder()?
            .profile("default")
            .build(BuildPolicy::Missing(None))
            .path(conan_file.to_str().context("None")?);

        let command = builder.into_command();
        let root = command.run()?;
        ConanInstall::link_options(&root, &["hyperscan::hs", "hyperscan::hs_runtime"])?;

        Ok(())
    }
}

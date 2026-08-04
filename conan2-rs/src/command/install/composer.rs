//! Conan install command composer to generate Conan install command line arguments.
//!

use crate::command::{BuildPolicy, ConanVerbosity};

/// # Conan Install Command composer
///
/// This file is generated with reference to the official Conan install command help.
///
/// The main options are also documented in the struct fields below.
#[derive(Default, Clone, Debug)]
pub struct InstallCommandArgs {
    /// Path to a folder containing a recipe (conanfile.py or conanfile.txt) or to a recipe file.
    pub path: Option<String>,
    /// Level of detail of the output. Valid options: -vquiet, -verror, -vwarning, -vnotice, -vstatus, -v, -vverbose, -vv, -vdebug, -vvv, -vtrace
    pub verbose: Option<ConanVerbosity>,
    /// Define core configuration, overwriting global.conf values. E.g.: -cc core:non_interactive=True
    pub core_conf: Vec<String>,
    /// Select the output format: json
    pub format: Option<String>,
    /// Write the output of the command to the specified file instead of stdout
    pub out_file: Option<String>,
    /// Provide a package name if not specified in conanfile
    pub name: Option<String>,
    /// Provide a package version if not specified in conanfile
    pub version: Option<String>,
    /// Provide a user if not specified in conanfile
    pub user: Option<String>,
    /// Provide a channel if not specified in conanfile
    pub channel: Option<String>,
    /// Directly provide requires instead of a conanfile
    pub requires: Vec<String>,
    /// Directly provide tool-requires instead of a conanfile
    pub tool_requires: Vec<String>,
    /// Specify which packages to build from source. See help for possible values
    pub build: Option<BuildPolicy>,
    /// Look in the specified remote or remotes server
    pub remotes: Vec<String>,
    /// Do not use remote, resolve exclusively in the cache
    pub no_remote: bool,
    /// Will install newer versions and/or revisions in the local cache for the given reference name, or all references in the graph if no argument is supplied
    pub update: bool,
    /// Apply the specified profile. By default, or if specifying -pr:h (--profile:host), it applies to the host context. Use -pr:b (--profile:build) to specify the build context, or -pr:a (--profile:all) to specify both contexts at once
    pub profile: Option<String>,
    /// Apply the specified build profile (-pr:b, --profile:build)
    pub profile_build: Option<String>,
    /// Apply the specified host profile (-pr:h, --profile:host)
    pub profile_host: Option<String>,
    /// Apply the specified all profile (-pr:a, --profile:all)
    pub profile_all: Option<String>,
    /// Apply the specified options. Example: -o="pkg/*:with_qt=True"
    pub options: Vec<String>,
    /// Apply the specified options for build context (-o:b, --options:build)
    pub options_build: Vec<String>,
    /// Apply the specified options for host context (-o:h, --options:host)
    pub options_host: Vec<String>,
    /// Apply the specified options for all contexts (-o:a, --options:all)
    pub options_all: Vec<String>,
    /// Apply the specified settings. Example: -s="compiler=gcc"
    pub settings: Vec<String>,
    /// Apply the specified settings for build context (-s:b, --settings:build)
    pub settings_build: Vec<String>,
    /// Apply the specified settings for host context (-s:h, --settings:host)
    pub settings_host: Vec<String>,
    /// Apply the specified settings for all contexts (-s:a, --settings:all)
    pub settings_all: Vec<String>,
    /// Apply the specified conf. Example: -c="tools.cmake.cmaketoolchain:generator=Xcode"
    pub conf: Vec<String>,
    /// Apply the specified conf for build context (-c:b, --conf:build)
    pub conf_build: Vec<String>,
    /// Apply the specified conf for host context (-c:h, --conf:host)
    pub conf_host: Vec<String>,
    /// Apply the specified conf for all contexts (-c:a, --conf:all)
    pub conf_all: Vec<String>,
    /// Path to a lockfile. Use --lockfile="" to avoid automatic use of existing 'conan.lock' file
    pub lockfile: Option<String>,
    /// Do not raise an error if some dependency is not found in lockfile
    pub lockfile_partial: bool,
    /// Filename of the updated lockfile
    pub lockfile_out: Option<String>,
    /// Remove unused entries from the lockfile
    pub lockfile_clean: bool,
    /// Overwrite lockfile overrides
    pub lockfile_overrides: Vec<String>,
    /// Generators to use
    pub generator: Option<String>,
    /// The root output folder for generated and build files
    pub output_folder: Option<String>,
    /// Deploy using the provided deployer to the output folder
    pub deployer: Option<String>,
    /// Deployer output folder, base build folder by default if not set
    pub deployer_folder: Option<String>,
    /// Execute the deploy() method of the packages matching the provided patterns
    pub deployer_package: Option<String>,
    /// Whether the provided path is a build-require
    pub build_require: bool,
    /// Generation strategy for virtual environment files for the root
    pub envs_generation: Option<String>,
}

impl InstallCommandArgs {
    pub fn new() -> Self {
        Self::default()
    }
    /// Set the path to a folder containing a recipe (conanfile.py or conanfile.txt) or to a recipe file.
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }
    /// Set the level of detail of the output. Valid options: -vquiet, -verror, -vwarning, -vnotice, -vstatus, -v, -vverbose, -vv, -vdebug, -vvv, -vtrace
    pub fn verbose(mut self, verbose: impl Into<ConanVerbosity>) -> Self {
        self.verbose = Some(verbose.into());
        self
    }
    /// Add a core configuration, overwriting global.conf values. E.g.: -cc core:non_interactive=True
    pub fn core_conf(mut self, conf: impl Into<String>) -> Self {
        self.core_conf.push(conf.into());
        self
    }
    /// Set the output format (e.g., json)
    pub fn format(mut self, format: impl Into<String>) -> Self {
        self.format = Some(format.into());
        self
    }
    /// Set the output file for the command output
    pub fn out_file(mut self, out_file: impl Into<String>) -> Self {
        self.out_file = Some(out_file.into());
        self
    }
    /// Set the package name if not specified in conanfile
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
    /// Set the package version if not specified in conanfile
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }
    /// Set the user if not specified in conanfile
    pub fn user(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }
    /// Set the channel if not specified in conanfile
    pub fn channel(mut self, channel: impl Into<String>) -> Self {
        self.channel = Some(channel.into());
        self
    }
    /// Add a requirement (package reference) instead of a conanfile
    pub fn requires(mut self, req: impl Into<String>) -> Self {
        self.requires.push(req.into());
        self
    }
    /// Add a tool requirement (package reference) instead of a conanfile
    pub fn tool_requires(mut self, req: impl Into<String>) -> Self {
        self.tool_requires.push(req.into());
        self
    }
    /// Specify which packages to build from source
    pub fn build(mut self, build: impl Into<BuildPolicy>) -> Self {
        self.build = Some(build.into());
        self
    }
    /// Add a remote server to look for packages
    pub fn remotes(mut self, remote: impl Into<String>) -> Self {
        self.remotes.push(remote.into());
        self
    }
    /// Set whether to use remote servers (true disables remote usage)
    pub fn no_remote(mut self, no_remote: bool) -> Self {
        self.no_remote = no_remote;
        self
    }
    /// Set whether to install newer versions and/or revisions in the local cache
    pub fn update(mut self, update: bool) -> Self {
        self.update = update;
        self
    }
    /// Set the profile (default or host)
    pub fn profile(mut self, profile: impl Into<String>) -> Self {
        self.profile = Some(profile.into());
        self
    }
    /// Set the build profile (-pr:b, --profile:build)
    pub fn profile_build(mut self, profile: impl Into<String>) -> Self {
        self.profile_build = Some(profile.into());
        self
    }
    /// Set the host profile (-pr:h, --profile:host)
    pub fn profile_host(mut self, profile: impl Into<String>) -> Self {
        self.profile_host = Some(profile.into());
        self
    }
    /// Set the all profile (-pr:a, --profile:all)
    pub fn profile_all(mut self, profile: impl Into<String>) -> Self {
        self.profile_all = Some(profile.into());
        self
    }
    /// Add an option (applies to host context by default)
    pub fn options(mut self, opt: impl Into<String>) -> Self {
        self.options.push(opt.into());
        self
    }
    /// Add an option for build context (-o:b, --options:build)
    pub fn options_build(mut self, opt: impl Into<String>) -> Self {
        self.options_build.push(opt.into());
        self
    }
    /// Add an option for host context (-o:h, --options:host)
    pub fn options_host(mut self, opt: impl Into<String>) -> Self {
        self.options_host.push(opt.into());
        self
    }
    /// Add an option for all contexts (-o:a, --options:all)
    pub fn options_all(mut self, opt: impl Into<String>) -> Self {
        self.options_all.push(opt.into());
        self
    }
    /// Add a setting (applies to host context by default)
    pub fn settings(mut self, setting: impl Into<String>) -> Self {
        self.settings.push(setting.into());
        self
    }
    /// Add a setting for build context (-s:b, --settings:build)
    pub fn settings_build(mut self, setting: impl Into<String>) -> Self {
        self.settings_build.push(setting.into());
        self
    }
    /// Add a setting for host context (-s:h, --settings:host)
    pub fn settings_host(mut self, setting: impl Into<String>) -> Self {
        self.settings_host.push(setting.into());
        self
    }
    /// Add a setting for all contexts (-s:a, --settings:all)
    pub fn settings_all(mut self, setting: impl Into<String>) -> Self {
        self.settings_all.push(setting.into());
        self
    }
    /// Add a conf (applies to host context by default)
    pub fn conf(mut self, conf: impl Into<String>) -> Self {
        self.conf.push(conf.into());
        self
    }
    /// Add a conf for build context (-c:b, --conf:build)
    pub fn conf_build(mut self, conf: impl Into<String>) -> Self {
        self.conf_build.push(conf.into());
        self
    }
    /// Add a conf for host context (-c:h, --conf:host)
    pub fn conf_host(mut self, conf: impl Into<String>) -> Self {
        self.conf_host.push(conf.into());
        self
    }
    /// Add a conf for all contexts (-c:a, --conf:all)
    pub fn conf_all(mut self, conf: impl Into<String>) -> Self {
        self.conf_all.push(conf.into());
        self
    }
    /// Set the path to a lockfile
    pub fn lockfile(mut self, lockfile: impl Into<String>) -> Self {
        self.lockfile = Some(lockfile.into());
        self
    }
    /// Set whether to allow missing dependencies in lockfile
    pub fn lockfile_partial(mut self, partial: bool) -> Self {
        self.lockfile_partial = partial;
        self
    }
    /// Set the filename of the updated lockfile
    pub fn lockfile_out(mut self, out: impl Into<String>) -> Self {
        self.lockfile_out = Some(out.into());
        self
    }
    /// Set whether to remove unused entries from the lockfile
    pub fn lockfile_clean(mut self, clean: bool) -> Self {
        self.lockfile_clean = clean;
        self
    }
    /// Add a lockfile override
    pub fn lockfile_overrides(mut self, override_: impl Into<String>) -> Self {
        self.lockfile_overrides.push(override_.into());
        self
    }
    /// Set the generator to use
    pub fn generator(mut self, generator: impl Into<String>) -> Self {
        self.generator = Some(generator.into());
        self
    }
    /// Set the root output folder for generated and build files
    pub fn output_folder(mut self, folder: impl Into<String>) -> Self {
        self.output_folder = Some(folder.into());
        self
    }
    /// Set the deployer to use
    pub fn deployer(mut self, deployer: impl Into<String>) -> Self {
        self.deployer = Some(deployer.into());
        self
    }
    /// Set the deployer output folder
    pub fn deployer_folder(mut self, folder: impl Into<String>) -> Self {
        self.deployer_folder = Some(folder.into());
        self
    }
    /// Set the deployer package pattern
    pub fn deployer_package(mut self, package: impl Into<String>) -> Self {
        self.deployer_package = Some(package.into());
        self
    }
    /// Set whether the provided path is a build-require
    pub fn build_require(mut self, build_require: bool) -> Self {
        self.build_require = build_require;
        self
    }
    /// Set the generation strategy for virtual environment files for the root
    pub fn envs_generation(mut self, envs: impl Into<String>) -> Self {
        self.envs_generation = Some(envs.into());
        self
    }
    pub fn to_args(&self) -> Vec<String> {
        let mut args = vec!["install".to_string()];
        if let Some(path) = &self.path {
            args.push(path.clone());
        }
        if let Some(verbose) = &self.verbose {
            args.push("-v".to_string());
            args.push(verbose.to_arg());
        }
        for cc in &self.core_conf {
            args.push("-cc".to_string());
            args.push(cc.clone());
        }
        if let Some(format) = &self.format {
            args.push("-f".to_string());
            args.push(format.clone());
        }
        if let Some(out_file) = &self.out_file {
            args.push("--out-file".to_string());
            args.push(out_file.clone());
        }
        if let Some(name) = &self.name {
            args.push("--name".to_string());
            args.push(name.clone());
        }
        if let Some(version) = &self.version {
            args.push("--version".to_string());
            args.push(version.clone());
        }
        if let Some(user) = &self.user {
            args.push("--user".to_string());
            args.push(user.clone());
        }
        if let Some(channel) = &self.channel {
            args.push("--channel".to_string());
            args.push(channel.clone());
        }
        for req in &self.requires {
            args.push("--requires".to_string());
            args.push(req.clone());
        }
        for treq in &self.tool_requires {
            args.push("--tool-requires".to_string());
            args.push(treq.clone());
        }
        if let Some(build) = &self.build {
            args.push(build.to_arg());
        }
        for r in &self.remotes {
            args.push("-r".to_string());
            args.push(r.clone());
        }
        if self.no_remote {
            args.push("-nr".to_string());
        }
        if self.update {
            args.push("-u".to_string());
        }
        if let Some(profile) = &self.profile {
            args.push("-pr".to_string());
            args.push(profile.clone());
        }
        if let Some(profile_build) = &self.profile_build {
            args.push("-pr:b".to_string());
            args.push(profile_build.clone());
        }
        if let Some(profile_host) = &self.profile_host {
            args.push("-pr:h".to_string());
            args.push(profile_host.clone());
        }
        if let Some(profile_all) = &self.profile_all {
            args.push("-pr:a".to_string());
            args.push(profile_all.clone());
        }
        for opt in &self.options {
            args.push("-o".to_string());
            args.push(opt.clone());
        }
        for opt in &self.options_build {
            args.push("-o:b".to_string());
            args.push(opt.clone());
        }
        for opt in &self.options_host {
            args.push("-o:h".to_string());
            args.push(opt.clone());
        }
        for opt in &self.options_all {
            args.push("-o:a".to_string());
            args.push(opt.clone());
        }
        for s in &self.settings {
            args.push("-s".to_string());
            args.push(s.clone());
        }
        for s in &self.settings_build {
            args.push("-s:b".to_string());
            args.push(s.clone());
        }
        for s in &self.settings_host {
            args.push("-s:h".to_string());
            args.push(s.clone());
        }
        for s in &self.settings_all {
            args.push("-s:a".to_string());
            args.push(s.clone());
        }
        for c in &self.conf {
            args.push("-c".to_string());
            args.push(c.clone());
        }
        for c in &self.conf_build {
            args.push("-c:b".to_string());
            args.push(c.clone());
        }
        for c in &self.conf_host {
            args.push("-c:h".to_string());
            args.push(c.clone());
        }
        for c in &self.conf_all {
            args.push("-c:a".to_string());
            args.push(c.clone());
        }
        if let Some(lockfile) = &self.lockfile {
            args.push("-l".to_string());
            args.push(lockfile.clone());
        }
        if self.lockfile_partial {
            args.push("--lockfile-partial".to_string());
        }
        if let Some(lockfile_out) = &self.lockfile_out {
            args.push("--lockfile-out".to_string());
            args.push(lockfile_out.clone());
        }
        if self.lockfile_clean {
            args.push("--lockfile-clean".to_string());
        }
        for lfo in &self.lockfile_overrides {
            args.push("--lockfile-overrides".to_string());
            args.push(lfo.clone());
        }
        if let Some(generator) = &self.generator {
            args.push("-g".to_string());
            args.push(generator.clone());
        }
        if let Some(output_folder) = &self.output_folder {
            args.push("-of".to_string());
            args.push(output_folder.clone());
        }
        if let Some(deployer) = &self.deployer {
            args.push("-d".to_string());
            args.push(deployer.clone());
        }
        if let Some(deployer_folder) = &self.deployer_folder {
            args.push("--deployer-folder".to_string());
            args.push(deployer_folder.clone());
        }
        if let Some(deployer_package) = &self.deployer_package {
            args.push("--deployer-package".to_string());
            args.push(deployer_package.clone());
        }
        if self.build_require {
            args.push("--build-require".to_string());
        }
        if let Some(envs_generation) = &self.envs_generation {
            args.push("--envs-generation".to_string());
            args.push(envs_generation.clone());
        }
        args
    }
}

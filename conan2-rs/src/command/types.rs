/// `conan` command verbosity level
///
/// Defines the level of detail of the Conan command output.
///
/// Enum variants correspond to the following options:
/// `-vquiet`, `-verror`, `-vwarning`, `-vnotice`, `-vstatus`,
/// `-v` or `-vverbose`, `-vv` or `-vdebug`, `-vvv` or `-vtrace`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConanVerbosity {
    /// `-vquiet`
    Quiet,
    /// `-verror`
    Error,
    /// `-vwarning`
    #[default]
    Warning,
    /// `-vnotice`
    Notice,
    /// `-vstatus`
    Status,
    /// `-v` or `-vverbose`
    Verbose,
    /// `-vv` or `-vdebug`
    Debug,
    /// `-vvv` of `-vtrace`
    Trace,
}

impl ConanVerbosity {
    pub fn to_arg(&self) -> String {
        match self {
            ConanVerbosity::Quiet => "-vquiet",
            ConanVerbosity::Error => "-verror",
            ConanVerbosity::Warning => "-vwarning",
            ConanVerbosity::Notice => "-vnotice",
            ConanVerbosity::Status => "-vstatus",
            ConanVerbosity::Verbose => "-v",
            ConanVerbosity::Debug => "-vdebug",
            ConanVerbosity::Trace => "-vtrace",
        }
        .to_string()
    }
}

/// Conan build policy
#[derive(Debug, Clone, PartialEq)]
pub enum BuildPolicy {
    /// Disallow build for all packages, use binary packages or fail if a binary package is not found,
    /// it cannot be combined with other '--build' options.
    Never,
    /// Build packages from source whose binary package is not found.
    /// `--build=missing:[pattern]` Build from source if a compatible    
    /// binary does not exist, only for packages matching pattern.
    Missing(Option<String>),
    /// Build packages from source that have at least one dependency being built from source.
    Cascade,
    /// Build packages from source whose package reference matches the pattern. The pattern uses
    /// 'fnmatch' style wildcards, so
    ///
    /// * '--build="*"' will build everything from source.
    /// * `--build=~[pattern]` Excluded packages, which will not be built from the source, whose
    /// package reference matches the pattern. The pattern uses 'fnmatch' style wildcards.
    Pattern(String),
    /// `--build=compatible:[pattern]` (Experimental) Build from source if a compatible binary does not exist, and the requested package is invalid, the
    /// closest package binary following the defined compatibility policies (method and compatibility.py)
    Compatible(String),
}

impl BuildPolicy {
    pub fn to_arg(&self) -> String {
        match self {
            BuildPolicy::Never => "--build=never".to_string(),
            BuildPolicy::Missing(pattern) => match pattern {
                Some(p) => format!("--build=missing:{}", p),
                None => "--build=missing".to_string(),
            },
            BuildPolicy::Cascade => "--build=cascade".to_string(),
            BuildPolicy::Pattern(pattern) => format!("--build={}", pattern),
            BuildPolicy::Compatible(pattern) => format!("--build=compatible:{}", pattern),
        }
    }
}

pub enum ConanScope {
    All(String),
    Host(String),
    Build(String),
}

impl ConanScope {
    pub fn to_arg(&self, prefix: &str) -> String {
        match self {
            ConanScope::All(v) => format!("{}:a={}", prefix, v),
            ConanScope::Host(v) => format!("{}:h={}", prefix, v),
            ConanScope::Build(v) => format!("{}:b={}", prefix, v),
        }
    }

    pub fn to_arg_long(&self, prefix: &str) -> String {
        match self {
            ConanScope::All(v) => format!("{}:all={}", prefix, v),
            ConanScope::Host(v) => format!("{}:host={}", prefix, v),
            ConanScope::Build(v) => format!("{}:build={}", prefix, v),
        }
    }
}

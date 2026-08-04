# Conan help

```bash
> conan install -h

usage: conan install [-h] [-v [V]] [-cc CORE_CONF] [-f FORMAT] [--out-file OUT_FILE]              
                     [--name NAME] [--version VERSION] [--user USER] [--channel CHANNEL]          
                     [--requires REQUIRES] [--tool-requires TOOL_REQUIRES] [-b BUILD]
                     [-r REMOTE | -nr] [-u [UPDATE]] [-pr PROFILE] [-pr:b PROFILE_BUILD]
                     [-pr:h PROFILE_HOST] [-pr:a PROFILE_ALL] [-o OPTIONS] [-o:b OPTIONS_BUILD]   
                     [-o:h OPTIONS_HOST] [-o:a OPTIONS_ALL] [-s SETTINGS] [-s:b SETTINGS_BUILD]   
                     [-s:h SETTINGS_HOST] [-s:a SETTINGS_ALL] [-c CONF] [-c:b CONF_BUILD]
                     [-c:h CONF_HOST] [-c:a CONF_ALL] [-l LOCKFILE] [--lockfile-partial]
                     [--lockfile-out LOCKFILE_OUT] [--lockfile-clean]
                     [--lockfile-overrides LOCKFILE_OVERRIDES] [-g GENERATOR]
                     [-of OUTPUT_FOLDER] [-d DEPLOYER] [--deployer-folder DEPLOYER_FOLDER]        
                     [--deployer-package DEPLOYER_PACKAGE] [--build-require]
                     [--envs-generation {false}]
                     [path]

Install the requirements specified in a recipe (conanfile.py or conanfile.txt).

It can also be used to install packages without a conanfile, using the
--requires and --tool-requires arguments.

If any requirement is not found in the local cache, it will iterate the remotes
looking for it. When the full dependency graph is computed, and all dependencies
recipes have been found, it will look for binary packages matching the current settings.
If no binary package is found for some or several dependencies, it will error,
unless the '--build' argument is used to build it from source.

After installation of packages, the generators and deployers will be called.

positional arguments:
  path                  Path to a folder containing a recipe (conanfile.py or conanfile.txt) or   
                        to a recipe file. e.g., ./my_project/conanfile.txt.

options:
  -h, --help            show this help message and exit
  -v [V]                Level of detail of the output. Valid options from less verbose to more    
                        verbose: -vquiet, -verror, -vwarning, -vnotice, -vstatus, -v or
                        -vverbose, -vv or -vdebug, -vvv or -vtrace
  -cc, --core-conf CORE_CONF
                        Define core configuration, overwriting global.conf values. E.g.: -cc      
                        core:non_interactive=True
  -f, --format FORMAT   Select the output format: json
  --out-file OUT_FILE   Write the output of the command to the specified file instead of stdout.  
  --name NAME           Provide a package name if not specified in conanfile
  --version VERSION     Provide a package version if not specified in conanfile
  --user USER           Provide a user if not specified in conanfile
  --channel CHANNEL     Provide a channel if not specified in conanfile
  --requires REQUIRES   Directly provide requires instead of a conanfile
  --tool-requires TOOL_REQUIRES
                        Directly provide tool-requires instead of a conanfile
  -b, --build BUILD     Optional, specify which packages to build from source. Combining
                        multiple '--build' options on one command line is allowed. Possible       
                        values: --build=never Disallow build for all packages, use binary
                        packages or fail if a binary package is not found, it cannot be combined  
                        with other '--build' options. --build=missing Build packages from source  
                        whose binary package is not found. --build=cascade Build packages from    
                        source that have at least one dependency being built from source.
                        --build=[pattern] Build packages from source whose package reference      
                        matches the pattern. The pattern uses 'fnmatch' style wildcards, so '--   
                        build="*"' will build everything from source. --build=~[pattern]
                        Excluded packages, which will not be built from the source, whose
                        package reference matches the pattern. The pattern uses 'fnmatch' style   
                        wildcards. --build=missing:[pattern] Build from source if a compatible    
                        binary does not exist, only for packages matching pattern.
                        --build=compatible:[pattern] (Experimental) Build from source if a        
                        compatible binary does not exist, and the requested package is invalid,   
                        the closest package binary following the defined compatibility policies   
                        (method and compatibility.py)
  -r, --remote REMOTE   Look in the specified remote or remotes server
  -nr, --no-remote      Do not use remote, resolve exclusively in the cache
  -u, --update [UPDATE]
                        Will install newer versions and/or revisions in the local cache for the   
                        given reference name, or all references in the graph if no argument is    
                        supplied. When using version ranges, it will install the latest version   
                        that satisfies the range. It will update to the latest revision for the   
                        resolved version range.
  -pr, --profile PROFILE
                        Apply the specified profile. By default, or if specifying -pr:h
                        (--profile:host), it applies to the host context. Use -pr:b
                        (--profile:build) to specify the build context, or -pr:a (--profile:all)  
                        to specify both contexts at once
  -pr:b, --profile:build PROFILE_BUILD
  -pr:h, --profile:host PROFILE_HOST
  -pr:a, --profile:all PROFILE_ALL
  -o, --options OPTIONS
                        Apply the specified options. By default, or if specifying -o:h
                        (--options:host), it applies to the host context. Use -o:b
                        (--options:build) to specify the build context, or -o:a (--options:all)   
                        to specify both contexts at once. Example: -o="pkg/*:with_qt=True"        
  -o:b, --options:build OPTIONS_BUILD
  -o:h, --options:host OPTIONS_HOST
  -o:a, --options:all OPTIONS_ALL
  -s, --settings SETTINGS
                        Apply the specified settings. By default, or if specifying -s:h
                        (--settings:host), it applies to the host context. Use -s:b
                        (--settings:build) to specify the build context, or -s:a
                        (--settings:all) to specify both contexts at once. Example:
                        -s="compiler=gcc"
  -s:b, --settings:build SETTINGS_BUILD
  -s:h, --settings:host SETTINGS_HOST
  -s:a, --settings:all SETTINGS_ALL
  -c, --conf CONF       Apply the specified conf. By default, or if specifying -c:h
                        (--conf:host), it applies to the host context. Use -c:b (--conf:build)    
                        to specify the build context, or -c:a (--conf:all) to specify both        
                        contexts at once. Example:
                        -c="tools.cmake.cmaketoolchain:generator=Xcode"
  -c:b, --conf:build CONF_BUILD
  -c:h, --conf:host CONF_HOST
  -c:a, --conf:all CONF_ALL
  -l, --lockfile LOCKFILE
                        Path to a lockfile. Use --lockfile="" to avoid automatic use of existing  
                        'conan.lock' file
  --lockfile-partial    Do not raise an error if some dependency is not found in lockfile
  --lockfile-out LOCKFILE_OUT
                        Filename of the updated lockfile
  --lockfile-clean      Remove unused entries from the lockfile
  --lockfile-overrides LOCKFILE_OVERRIDES
                        Overwrite lockfile overrides
  -g, --generator GENERATOR
                        Generators to use
  -of, --output-folder OUTPUT_FOLDER
                        The root output folder for generated and build files
  -d, --deployer DEPLOYER
                        Deploy using the provided deployer to the output folder. Built-in
                        deployers: 'full_deploy', 'direct_deploy', 'runtime_deploy'
  --deployer-folder DEPLOYER_FOLDER
                        Deployer output folder, base build folder by default if not set
  --deployer-package DEPLOYER_PACKAGE
                        Execute the deploy() method of the packages matching the provided
                        patterns
  --build-require       Whether the provided path is a build-require
  --envs-generation {false}
                        Generation strategy for virtual environment files for the root
```

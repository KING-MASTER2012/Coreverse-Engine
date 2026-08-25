# Configuration Files

## tool-versions.json
The target minimum version for each toolchain tool. `bootstrap.ps1` / `bootstrap.sh` reads this file and
passes it to each `check-*` script as `-RequiredVersion`. It is sufficient to upgrade the version here;
there is no need to modify the script code.

## project-paths.json
The file that **must be updated** whenever changes are made to the actual directory structure of the Coreverse Engine repository.

- `cargoWorkspaceRoot` : The directory containing the root `Cargo.toml` (workspace).
- `vcpkgManifestDir`   : The directory containing the `vcpkg.json` manifest file.
- `vcpkgInstallDir`    : The directory where vcpkg will be cloned locally for the project (not committed to git;
  should be added to `.gitignore`).
- `cmakeSourceDir` / `cmakeBuildDir` : Used for the final `cmake -S ... -B ...` invocation.

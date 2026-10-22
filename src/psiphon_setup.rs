use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const CLIENT_NAME: &str = "psiphon-tunnel-core";
const BUILD_SCRIPT: &str = include_str!("../vendor/Aether/psiphon-build.sh");

pub fn managed_client_path() -> PathBuf {
    managed_pt_dir().join(format!("{CLIENT_NAME}{}", std::env::consts::EXE_SUFFIX))
}

pub fn managed_pt_dir() -> PathBuf {
    glib::user_data_dir().join("aether-gtk").join("pt")
}

pub fn installed_client() -> Option<PathBuf> {
    let path = managed_client_path();
    is_executable_client(&path).then_some(path)
}

pub fn is_executable_client(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

pub fn install_client() -> Result<PathBuf, String> {
    require_tool("go", &["version"], "Install Go 1.22 or newer")?;
    require_tool("git", &["--version"], "Install Git")?;

    let (goos, goarch, goarm) = host_target()?;
    build_client_from_script(
        &glib::user_data_dir().join("aether-gtk"),
        BUILD_SCRIPT,
        goos,
        goarch,
        goarm,
    )
}

fn require_tool(program: &str, args: &[&str], remedy: &str) -> Result<(), String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|_| format!("Psiphon setup requires {program}. {remedy}, then try again."))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "Could not run {program} for Psiphon setup. {remedy}."
        ))
    }
}

fn host_target() -> Result<(&'static str, &'static str, Option<&'static str>), String> {
    if std::env::consts::OS != "linux" {
        return Err("The in-app Psiphon builder currently supports Linux only.".into());
    }

    let (goarch, goarm) = match std::env::consts::ARCH {
        "x86_64" => ("amd64", None),
        "aarch64" => ("arm64", None),
        "x86" => ("386", None),
        "arm" => ("arm", Some("7")),
        architecture => {
            return Err(format!(
                "Psiphon setup does not yet support the {architecture} architecture."
            ));
        }
    };
    Ok(("linux", goarch, goarm))
}

fn build_client_from_script(
    data_dir: &Path,
    script: &str,
    goos: &str,
    goarch: &str,
    goarm: Option<&str>,
) -> Result<PathBuf, String> {
    fs::create_dir_all(data_dir)
        .map_err(|error| format!("Could not create {}: {error}", data_dir.display()))?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let staging = data_dir.join(format!(".psiphon-build-{}-{stamp}", std::process::id()));
    fs::create_dir(&staging)
        .map_err(|error| format!("Could not create Psiphon build staging directory: {error}"))?;
    let staging = fs::canonicalize(staging)
        .map_err(|error| format!("Could not resolve Psiphon build staging path: {error}"))?;

    let result = (|| {
        let script_path = staging.join("psiphon-build.sh");
        let output_dir = staging.join("pt");
        fs::write(&script_path, script)
            .map_err(|error| format!("Could not prepare the Psiphon build script: {error}"))?;

        let mut command = Command::new("bash");
        command
            .arg(&script_path)
            .arg(goos)
            .arg(goarch)
            .arg(&output_dir);
        if let Some(goarm) = goarm {
            command.arg(goarm);
        }
        let output = command.output().map_err(|error| {
            format!("Could not start the Psiphon build script. Install Bash and retry: {error}")
        })?;
        if !output.status.success() {
            let details = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            let details = if details.is_empty() {
                String::from_utf8_lossy(&output.stdout).trim().to_owned()
            } else {
                details
            };
            return Err(if details.is_empty() {
                format!("Psiphon build failed with {}.", output.status)
            } else {
                format!("Psiphon build failed: {details}")
            });
        }

        let staged_client =
            output_dir.join(format!("{CLIENT_NAME}{}", std::env::consts::EXE_SUFFIX));
        if !is_executable_client(&staged_client) {
            return Err(
                "The Psiphon build finished without producing an executable client.".into(),
            );
        }

        let installed = managed_client_path_under(data_dir);
        let parent = installed
            .parent()
            .ok_or_else(|| "Could not determine the Psiphon install directory".to_owned())?;
        fs::create_dir_all(parent)
            .map_err(|error| format!("Could not create {}: {error}", parent.display()))?;
        fs::rename(&staged_client, &installed).map_err(|error| {
            format!(
                "Could not install the Psiphon client at {}: {error}",
                installed.display()
            )
        })?;
        Ok(installed)
    })();

    let _ = fs::remove_dir_all(&staging);
    result
}

fn managed_client_path_under(data_dir: &Path) -> PathBuf {
    data_dir
        .join("pt")
        .join(format!("{CLIENT_NAME}{}", std::env::consts::EXE_SUFFIX))
}

pub fn resolve_client_path(
    explicit: Option<&Path>,
    managed: &Path,
) -> Result<Option<PathBuf>, String> {
    if let Some(explicit) = explicit {
        if !is_executable_client(explicit) {
            return Err(format!(
                "The selected Psiphon client is not executable: {}",
                explicit.display()
            ));
        }
        return Ok(Some(explicit.to_path_buf()));
    }
    Ok(is_executable_client(managed).then(|| managed.to_path_buf()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(label: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("aether-gtk-{label}-{}-{stamp}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn write_executable(path: &Path) {
        fs::write(path, "#!/bin/sh\nexit 0\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
        }
    }

    #[test]
    fn explicit_client_path_wins_and_unset_path_uses_managed_client() {
        let root = temp_dir("client-choice");
        let managed = root.join("managed-client");
        let explicit = root.join("custom-client");
        write_executable(&managed);
        write_executable(&explicit);

        assert_eq!(
            resolve_client_path(Some(&explicit), &managed).unwrap(),
            Some(explicit)
        );
        assert_eq!(resolve_client_path(None, &managed).unwrap(), Some(managed));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn build_publishes_only_a_valid_executable_and_cleans_staging() {
        let root = temp_dir("client-build");
        let script = r#"mkdir -p "$3"; printf '#!/bin/sh\nexit 0\n' > "$3/psiphon-tunnel-core"; chmod +x "$3/psiphon-tunnel-core""#;

        let installed = build_client_from_script(&root, script, "linux", "amd64", None).unwrap();
        assert!(is_executable_client(&installed));
        assert_eq!(installed, managed_client_path_under(&root));
        assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn failed_or_incomplete_build_leaves_no_installed_client_or_staging_files() {
        let root = temp_dir("client-build-failure");
        let failure = build_client_from_script(
            &root,
            "echo build failed >&2; exit 2",
            "linux",
            "amd64",
            None,
        );
        assert!(failure.unwrap_err().contains("build failed"));

        let incomplete = build_client_from_script(&root, "mkdir -p \"$3\"", "linux", "amd64", None);
        assert!(incomplete
            .unwrap_err()
            .contains("without producing an executable"));
        assert!(!managed_client_path_under(&root).exists());
        assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn non_executable_files_are_rejected() {
        let root = temp_dir("client-permissions");
        let file = root.join(CLIENT_NAME);
        fs::write(&file, "not executable").unwrap();
        assert!(!is_executable_client(&file));
        assert!(is_executable_client(Path::new("/bin/sh")));
        let _ = fs::remove_dir_all(root);
    }
}

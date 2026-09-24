use crate::sanitize_name;
use std::collections::HashMap;
use std::env;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) struct Workspace {
    pub(crate) name: String,
    pub(crate) mounts: Vec<Mount>,
    pub(crate) docker_ports: Option<PortRange>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PortRange {
    pub(crate) start: u16,
    pub(crate) end: u16,
}

impl PortRange {
    fn overlaps(self, other: Self) -> bool {
        self.start <= other.end && other.start <= self.end
    }
}

impl fmt::Display for PortRange {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}-{}", self.start, self.end)
    }
}

pub(crate) struct Mount {
    pub(crate) name: String,
    pub(crate) host: PathBuf,
}

pub(crate) struct Defaults {
    pub(crate) claude_home: Option<PathBuf>,
    pub(crate) codex_home: Option<PathBuf>,
    pub(crate) docker: bool,
}

pub(crate) struct Config {
    pub(crate) defaults: Defaults,
    pub(crate) workspaces: Vec<Workspace>,
}

pub(crate) fn load_config() -> Result<Config, String> {
    let config_path = env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".config").join("ai-sandbox").join("box.toml"))
        .ok_or_else(|| {
            "cannot locate ~/.config/ai-sandbox/box.toml; HOME is not set".to_string()
        })?;
    let contents = fs::read_to_string(&config_path)
        .map_err(|error| format!("cannot read {}: {error}", config_path.display()))?;
    parse_config(&config_path, &contents)
}

fn parse_config(config_path: &Path, contents: &str) -> Result<Config, String> {
    enum Section {
        None,
        Defaults,
        Workspace,
        Mount,
    }

    let mut defaults = Defaults {
        claude_home: None,
        codex_home: None,
        docker: true,
    };
    let mut workspaces = Vec::new();
    let mut current_workspace: Option<Workspace> = None;
    let mut current_mount: Option<Mount> = None;
    let mut section = Section::None;

    for (line_number, line) in contents.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        match trimmed {
            "[defaults]" => {
                finish_mount(&mut current_workspace, &mut current_mount)?;
                finish_workspace(&mut workspaces, &mut current_workspace);
                section = Section::Defaults;
                continue;
            }
            "[[workspaces]]" => {
                finish_mount(&mut current_workspace, &mut current_mount)?;
                finish_workspace(&mut workspaces, &mut current_workspace);
                current_workspace = Some(Workspace {
                    name: String::new(),
                    mounts: Vec::new(),
                    docker_ports: None,
                });
                section = Section::Workspace;
                continue;
            }
            "[[workspaces.mounts]]" => {
                finish_mount(&mut current_workspace, &mut current_mount)?;
                if current_workspace.is_none() {
                    return Err(format!(
                        "{}:{}: mount defined before workspace",
                        config_path.display(),
                        line_number + 1
                    ));
                }
                current_mount = Some(Mount {
                    name: String::new(),
                    host: PathBuf::new(),
                });
                section = Section::Mount;
                continue;
            }
            _ => {}
        }

        let Some((key, value)) = trimmed.split_once('=') else {
            return Err(format!(
                "{}:{}: expected key = value",
                config_path.display(),
                line_number + 1
            ));
        };

        let key = key.trim();
        let value = parse_value(value.trim());

        match section {
            Section::Defaults => match key {
                "claude_home" => defaults.claude_home = Some(PathBuf::from(expand_home(&value))),
                "codex_home" => defaults.codex_home = Some(PathBuf::from(expand_home(&value))),
                "docker" => defaults.docker = parse_bool(config_path, line_number + 1, &value)?,
                _ => return Err(unknown_key(config_path, line_number + 1, key)),
            },
            Section::Workspace => {
                let workspace = current_workspace.as_mut().ok_or_else(|| {
                    format!(
                        "{}:{}: workspace key outside workspace section",
                        config_path.display(),
                        line_number + 1
                    )
                })?;
                match key {
                    "name" => workspace.name = value,
                    "docker_ports" => {
                        workspace.docker_ports =
                            Some(parse_port_range(config_path, line_number + 1, &value)?)
                    }
                    _ => return Err(unknown_key(config_path, line_number + 1, key)),
                }
            }
            Section::Mount => {
                let mount = current_mount.as_mut().ok_or_else(|| {
                    format!(
                        "{}:{}: mount key outside mount section",
                        config_path.display(),
                        line_number + 1
                    )
                })?;
                match key {
                    "name" => mount.name = value,
                    "host" => mount.host = PathBuf::from(expand_home(&value)),
                    _ => return Err(unknown_key(config_path, line_number + 1, key)),
                }
            }
            Section::None => {
                return Err(format!(
                    "{}:{}: key outside a section",
                    config_path.display(),
                    line_number + 1
                ));
            }
        }
    }

    finish_mount(&mut current_workspace, &mut current_mount)?;
    finish_workspace(&mut workspaces, &mut current_workspace);
    validate_config(config_path, &workspaces)?;

    Ok(Config {
        defaults,
        workspaces,
    })
}

fn validate_config(config_path: &Path, workspaces: &[Workspace]) -> Result<(), String> {
    if workspaces.is_empty() {
        return Err(format!(
            "no workspaces configured in {}",
            config_path.display()
        ));
    }

    let mut runtime_names = HashMap::new();
    let mut workspace_port_ranges = Vec::new();
    for workspace in workspaces {
        if workspace.name.is_empty() {
            return Err(format!(
                "{}: workspace name is required",
                config_path.display()
            ));
        }
        let runtime_name = sanitize_name(&workspace.name).to_ascii_lowercase();
        if let Some(existing_name) = runtime_names.insert(runtime_name.clone(), &workspace.name) {
            return Err(format!(
                "{}: workspace names '{}' and '{}' normalize to the same runtime name '{}'",
                config_path.display(),
                existing_name,
                workspace.name,
                runtime_name
            ));
        }
        if workspace.mounts.is_empty() {
            return Err(format!(
                "{}: workspace '{}' has no mounts",
                config_path.display(),
                workspace.name
            ));
        }
        for mount in &workspace.mounts {
            if mount.name.is_empty() || !mount.host.is_absolute() {
                return Err(format!(
                    "{}: mount in workspace '{}' requires name and absolute host path",
                    config_path.display(),
                    workspace.name
                ));
            }
        }
        if let Some(port_range) = workspace.docker_ports {
            if let Some((existing_name, existing_range)) = workspace_port_ranges
                .iter()
                .find(|(_, existing_range)| port_range.overlaps(*existing_range))
            {
                return Err(format!(
                    "{}: docker port ranges for workspaces '{}' ({}) and '{}' ({}) overlap",
                    config_path.display(),
                    existing_name,
                    existing_range,
                    workspace.name,
                    port_range
                ));
            }
            workspace_port_ranges.push((&workspace.name, port_range));
        }
    }

    Ok(())
}

fn finish_workspace(workspaces: &mut Vec<Workspace>, current_workspace: &mut Option<Workspace>) {
    if let Some(workspace) = current_workspace.take() {
        workspaces.push(workspace);
    }
}

fn finish_mount(
    current_workspace: &mut Option<Workspace>,
    current_mount: &mut Option<Mount>,
) -> Result<(), String> {
    if let Some(mount) = current_mount.take() {
        let workspace = current_workspace
            .as_mut()
            .ok_or_else(|| "mount defined before workspace".to_string())?;
        workspace.mounts.push(mount);
    }
    Ok(())
}

fn parse_value(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_string()
}

fn parse_bool(config_path: &Path, line_number: usize, value: &str) -> Result<bool, String> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!(
            "{}:{}: expected true or false",
            config_path.display(),
            line_number
        )),
    }
}

fn parse_port_range(
    config_path: &Path,
    line_number: usize,
    value: &str,
) -> Result<PortRange, String> {
    let Some((start, end)) = value.split_once('-') else {
        return Err(format!(
            "{}:{}: expected docker port range in START-END format",
            config_path.display(),
            line_number
        ));
    };
    let parse_port = |port: &str| {
        port.parse::<u16>().map_err(|_| {
            format!(
                "{}:{}: invalid Docker port '{}'",
                config_path.display(),
                line_number,
                port
            )
        })
    };
    let start = parse_port(start)?;
    let end = parse_port(end)?;
    if start < 1024 || start > end {
        return Err(format!(
            "{}:{}: Docker port range must be ordered and use ports 1024-65535",
            config_path.display(),
            line_number
        ));
    }
    Ok(PortRange { start, end })
}

fn unknown_key(config_path: &Path, line_number: usize, key: &str) -> String {
    format!(
        "{}:{}: unknown key '{}'",
        config_path.display(),
        line_number,
        key
    )
}

fn expand_home(path: &str) -> String {
    if path == "~" {
        return env::var("HOME").unwrap_or_else(|_| path.to_string());
    }
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = env::var("HOME") {
            return format!("{home}/{rest}");
        }
    }
    path.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_colliding_runtime_names() {
        let contents = r#"
[[workspaces]]
name = "API"

[[workspaces.mounts]]
name = "service-a"
host = "/workspace/service-a"

[[workspaces]]
name = "api"

[[workspaces.mounts]]
name = "service-b"
host = "/workspace/service-b"
"#;

        let error = match parse_config(Path::new("box.toml"), contents) {
            Ok(_) => panic!("colliding workspace names should be rejected"),
            Err(error) => error,
        };

        assert!(error.contains("'API' and 'api' normalize to the same runtime name 'api'"));
    }

    #[test]
    fn parses_workspace_docker_port_range() {
        let contents = r#"
[[workspaces]]
name = "api"
docker_ports = "18000-18099"

[[workspaces.mounts]]
name = "api"
host = "/workspace/api"
"#;

        let config = parse_config(Path::new("box.toml"), contents).unwrap();

        assert_eq!(
            config.workspaces[0].docker_ports,
            Some(PortRange {
                start: 18000,
                end: 18099
            })
        );
    }

    #[test]
    fn rejects_overlapping_workspace_docker_port_ranges() {
        let contents = r#"
[[workspaces]]
name = "api"
docker_ports = "18000-18099"

[[workspaces.mounts]]
name = "api"
host = "/workspace/api"

[[workspaces]]
name = "web"
docker_ports = "18099-18199"

[[workspaces.mounts]]
name = "web"
host = "/workspace/web"
"#;

        let error = match parse_config(Path::new("box.toml"), contents) {
            Ok(_) => panic!("overlapping Docker port ranges should be rejected"),
            Err(error) => error,
        };

        assert!(error.contains("docker port ranges for workspaces 'api'"));
        assert!(error.contains("and 'web' (18099-18199) overlap"));
    }

    #[test]
    fn rejects_privileged_docker_port_range() {
        let contents = r#"
[[workspaces]]
name = "api"
docker_ports = "80-100"

[[workspaces.mounts]]
name = "api"
host = "/workspace/api"
"#;

        let error = match parse_config(Path::new("box.toml"), contents) {
            Ok(_) => panic!("privileged Docker ports should be rejected"),
            Err(error) => error,
        };

        assert!(error.contains("ports 1024-65535"));
    }
}

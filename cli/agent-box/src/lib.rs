use dialoguer::{theme::ColorfulTheme, FuzzySelect};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Clone, Copy)]
pub enum Agent {
    Claude,
    Codex,
}

impl Agent {
    fn command(self) -> &'static str {
        match self {
            Agent::Claude => "claude",
            Agent::Codex => "codex",
        }
    }

    fn service(self) -> &'static str {
        match self {
            Agent::Claude => "claude-agent",
            Agent::Codex => "codex-agent",
        }
    }

    fn home_env(self) -> &'static str {
        match self {
            Agent::Claude => "CLAUDE_HOME_PATH",
            Agent::Codex => "CODEX_HOME_PATH",
        }
    }

    fn container_name_env(self) -> &'static str {
        match self {
            Agent::Claude => "CLAUDE_CONTAINER_NAME",
            Agent::Codex => "CODEX_CONTAINER_NAME",
        }
    }

    fn entrypoint(self) -> &'static str {
        match self {
            Agent::Claude => "/usr/local/bin/claude-entrypoint.sh",
            Agent::Codex => "/usr/local/bin/codex-entrypoint.sh",
        }
    }

    fn default_home(self) -> Option<PathBuf> {
        let home = env::var_os("HOME").map(PathBuf::from)?;
        Some(match self {
            Agent::Claude => home.join(".claude"),
            Agent::Codex => home.join(".codex"),
        })
    }

    fn compose_files(self, docker_enabled: bool) -> Vec<&'static str> {
        let mut files = vec!["docker-compose.yml"];
        if docker_enabled {
            files.push("docker-compose.docker.yml");
        }
        files
    }
}

struct Workspace {
    name: String,
    mounts: Vec<Mount>,
}

struct Mount {
    name: String,
    host: PathBuf,
}

struct Defaults {
    claude_home: Option<PathBuf>,
    codex_home: Option<PathBuf>,
    docker: bool,
}

struct Config {
    defaults: Defaults,
    workspaces: Vec<Workspace>,
}

pub fn run(agent: Agent) -> i32 {
    match run_inner(agent) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("error: {error}");
            1
        }
    }
}

fn run_inner(agent: Agent) -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        print_help(agent);
        return Ok(());
    }

    let cli_no_docker = args.iter().any(|arg| arg == "--no-docker");
    let rebuild = args.iter().any(|arg| arg == "--build");
    let sandbox_dir = find_sandbox_dir()?;
    let config = load_config(&sandbox_dir)?;
    let docker_enabled = config.defaults.docker && !cli_no_docker;
    let workspace = choose_workspace(&config.workspaces)?;
    let mount = choose_mount(workspace)?;
    let project_name = runtime_project_name(workspace, docker_enabled);
    let override_path = write_compose_override(agent, &sandbox_dir, workspace, docker_enabled)?;

    start_shared_proxy(&sandbox_dir, rebuild)?;
    start_stack(
        agent,
        &sandbox_dir,
        &override_path,
        &config.defaults,
        &project_name,
        docker_enabled,
        rebuild,
    )?;
    exec_agent(
        agent,
        &sandbox_dir,
        &override_path,
        &config.defaults,
        &project_name,
        mount,
        docker_enabled,
    )
}

fn print_help(agent: Agent) {
    println!("Usage: {} [--no-docker] [--build]", agent_box_name(agent));
    println!();
    println!("Reads workspace definitions from box.toml in the sandbox directory.");
    println!("Create one with: cp box.toml.example box.toml");
    println!("By default, starts existing images with docker compose up -d.");
    println!("Pass --build to rebuild images before opening the agent.");
}

fn agent_box_name(agent: Agent) -> &'static str {
    match agent {
        Agent::Claude => "claude-box",
        Agent::Codex => "codex-box",
    }
}

fn find_sandbox_dir() -> Result<PathBuf, String> {
    if let Some(path) = env::var_os("AI_SANDBOX_DIR") {
        return Ok(PathBuf::from(path));
    }

    let mut current = env::current_dir().map_err(|error| error.to_string())?;
    loop {
        if current.join("docker-compose.yml").is_file() && current.join("Makefile").is_file() {
            return Ok(current);
        }
        if !current.pop() {
            break;
        }
    }

    Err("run from ai-sandbox or set AI_SANDBOX_DIR".to_string())
}

fn load_config(sandbox_dir: &Path) -> Result<Config, String> {
    let config_path = sandbox_dir.join("box.toml");
    if !config_path.exists() {
        return Err(format!(
            "{} not found; create it with: cp box.toml.example box.toml",
            config_path.display()
        ));
    }

    let contents = fs::read_to_string(&config_path).map_err(|error| error.to_string())?;
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
                finish_workspace(&mut workspaces, &mut current_workspace)?;
                section = Section::Defaults;
                continue;
            }
            "[[workspaces]]" => {
                finish_mount(&mut current_workspace, &mut current_mount)?;
                finish_workspace(&mut workspaces, &mut current_workspace)?;
                current_workspace = Some(Workspace {
                    name: String::new(),
                    mounts: Vec::new(),
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
    finish_workspace(&mut workspaces, &mut current_workspace)?;

    if workspaces.is_empty() {
        return Err(format!(
            "no workspaces configured in {}",
            config_path.display()
        ));
    }

    let mut runtime_names = HashMap::new();
    for workspace in &workspaces {
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
    }

    Ok(Config {
        defaults,
        workspaces,
    })
}

fn finish_workspace(
    workspaces: &mut Vec<Workspace>,
    current_workspace: &mut Option<Workspace>,
) -> Result<(), String> {
    if let Some(workspace) = current_workspace.take() {
        workspaces.push(workspace);
    }
    Ok(())
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

fn unknown_key(config_path: &Path, line_number: usize, key: &str) -> String {
    format!(
        "{}:{}: unknown key '{}'",
        config_path.display(),
        line_number,
        key
    )
}

fn write_compose_override(
    agent: Agent,
    sandbox_dir: &Path,
    workspace: &Workspace,
    docker_enabled: bool,
) -> Result<PathBuf, String> {
    let output_dir = sandbox_dir.join(".tmp").join("agent-box");
    fs::create_dir_all(&output_dir).map_err(|error| error.to_string())?;

    let output_path = output_dir.join(format!(
        "{}-{}.yml",
        agent.command(),
        sanitize_name(&workspace.name)
    ));

    let mut contents = String::from("services:\n");
    write_service_mounts(&mut contents, agent.service(), &workspace.mounts);
    contents.push_str("    command: [\"sleep\", \"infinity\"]\n");
    if docker_enabled {
        contents.push_str(
            "    depends_on: !override\n      docker-daemon:\n        condition: service_healthy\n",
        );
        write_service_mounts(&mut contents, "docker-daemon", &workspace.mounts);
        contents.push_str(
            "    depends_on: !override\n      docker-daemon-init:\n        condition: service_completed_successfully\n",
        );
    } else {
        contents.push_str("    depends_on: !reset {}\n");
    }
    contents.push_str(
        "networks:\n  agent_net: !override\n    external: true\n    name: ai-sandbox_agent_net\n",
    );

    fs::write(&output_path, contents).map_err(|error| error.to_string())?;
    Ok(output_path)
}

fn write_service_mounts(contents: &mut String, service: &str, mounts: &[Mount]) {
    contents.push_str(&format!("  {service}:\n"));
    contents.push_str("    volumes:\n");
    for mount in mounts {
        contents.push_str(&format!(
            "      - type: bind\n        source: {}\n        target: /home/devops/project/{}\n",
            yaml_path(&mount.host),
            yaml_segment(&mount.name)
        ));
    }
}

fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '-'
            }
        })
        .collect()
}

fn runtime_project_name(workspace: &Workspace, docker_enabled: bool) -> String {
    let workspace_name = sanitize_name(&workspace.name).to_ascii_lowercase();
    let mode = if docker_enabled { "docker" } else { "plain" };
    format!("agent-box-{workspace_name}-{mode}")
}

fn start_shared_proxy(sandbox_dir: &Path, rebuild: bool) -> Result<(), String> {
    run_command(shared_proxy_command(sandbox_dir, rebuild))
}

fn shared_proxy_command(sandbox_dir: &Path, rebuild: bool) -> Command {
    let mut command = Command::new("docker");
    command
        .current_dir(sandbox_dir)
        .arg("compose")
        .arg("--project-name")
        .arg("ai-sandbox")
        .arg("-f")
        .arg("docker-compose.yml")
        .arg("up")
        .arg("-d")
        .arg("--wait");
    if rebuild {
        command.arg("--build");
    }
    command.arg("proxy");
    command
}

fn yaml_path(path: &Path) -> String {
    yaml_quote(&path.display().to_string())
}

fn yaml_segment(value: &str) -> String {
    value.replace(':', "-")
}

fn yaml_quote(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
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

fn choose_workspace(workspaces: &[Workspace]) -> Result<&Workspace, String> {
    if workspaces.len() == 1 {
        return Ok(&workspaces[0]);
    }

    let options = workspaces
        .iter()
        .map(|workspace| {
            let mounts = workspace
                .mounts
                .iter()
                .map(|mount| mount.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            format!("{:<18} {}", workspace.name, mounts)
        })
        .collect::<Vec<_>>();
    let selected = choose_from_menu("Workspace", &options)?;
    Ok(&workspaces[selected])
}

fn choose_mount(workspace: &Workspace) -> Result<&Mount, String> {
    if workspace.mounts.len() == 1 {
        return Ok(&workspace.mounts[0]);
    }

    let options = format_mount_options(&workspace.mounts);

    let selected = choose_from_menu(&format!("{} context", workspace.name), &options)?;
    Ok(&workspace.mounts[selected])
}

fn format_mount_options(mounts: &[Mount]) -> Vec<String> {
    let width = mounts
        .iter()
        .map(|mount| mount.name.len())
        .max()
        .unwrap_or(0)
        .max(8);

    mounts
        .iter()
        .map(|mount| {
            format!(
                "{:<width$} {}",
                mount.name,
                mount.host.display(),
                width = width
            )
        })
        .collect()
}

fn choose_from_menu(title: &str, options: &[String]) -> Result<usize, String> {
    if options.is_empty() {
        return Err("menu has no options".to_string());
    }

    FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt(title)
        .items(options)
        .default(0)
        .interact_opt()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "selection cancelled".to_string())
}

fn start_stack(
    agent: Agent,
    sandbox_dir: &Path,
    override_path: &Path,
    defaults: &Defaults,
    project_name: &str,
    docker_enabled: bool,
    rebuild: bool,
) -> Result<(), String> {
    let mut command = Command::new("docker");
    command.current_dir(sandbox_dir);
    command
        .arg("compose")
        .arg("--project-name")
        .arg(project_name);
    for file in agent.compose_files(docker_enabled) {
        command.arg("-f").arg(file);
    }
    command.arg("-f").arg(override_path).arg("up").arg("-d");
    if rebuild {
        command.arg("--build");
    }
    command.arg(agent.service());
    set_home_env(agent, &mut command);
    set_configured_home_env(agent, defaults, &mut command);
    set_runtime_env(agent, project_name, docker_enabled, &mut command);
    run_command(command)
}

fn exec_agent(
    agent: Agent,
    sandbox_dir: &Path,
    override_path: &Path,
    defaults: &Defaults,
    project_name: &str,
    mount: &Mount,
    docker_enabled: bool,
) -> Result<(), String> {
    let mut command = Command::new("docker");
    command.current_dir(sandbox_dir);
    command
        .arg("compose")
        .arg("--project-name")
        .arg(project_name);
    for file in agent.compose_files(docker_enabled) {
        command.arg("-f").arg(file);
    }
    command.arg("-f").arg(override_path);
    command
        .arg("exec")
        .arg("-e")
        .arg("HOME=/home/devops")
        .arg("-w")
        .arg(container_context(&mount.name))
        .arg(agent.service())
        .arg(agent.entrypoint())
        .arg(agent.command());
    set_home_env(agent, &mut command);
    set_configured_home_env(agent, defaults, &mut command);
    set_runtime_env(agent, project_name, docker_enabled, &mut command);
    run_command(command)
}

fn container_context(mount_name: &str) -> String {
    format!("/home/devops/project/{}", yaml_segment(mount_name))
}

fn set_home_env(agent: Agent, command: &mut Command) {
    if env::var_os(agent.home_env()).is_some() {
        return;
    }
    if let Some(default_home) = agent.default_home() {
        command.env(agent.home_env(), default_home);
    }
}

fn set_configured_home_env(agent: Agent, defaults: &Defaults, command: &mut Command) {
    if env::var_os(agent.home_env()).is_some() {
        return;
    }

    let configured = match agent {
        Agent::Claude => defaults.claude_home.as_ref(),
        Agent::Codex => defaults.codex_home.as_ref(),
    };
    if let Some(path) = configured {
        command.env(agent.home_env(), path);
    }
}

fn set_runtime_env(agent: Agent, project_name: &str, docker_enabled: bool, command: &mut Command) {
    command.env("COMPOSE_IGNORE_ORPHANS", "true");
    command.env("COMPOSE_PROFILES", agent.command());
    command.env(
        agent.container_name_env(),
        format!("{project_name}-{}", agent.service()),
    );
    if docker_enabled {
        command.env(
            "AGENT_DIND_DATA_VOLUME",
            format!("{project_name}-dind-data"),
        );
        command.env(
            "AGENT_DIND_SOCK_VOLUME",
            format!("{project_name}-dind-sock"),
        );
    }
}

fn run_command(mut command: Command) -> Result<(), String> {
    let status = command
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|error| error.to_string())?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("command exited with {status}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace(name: &str) -> Workspace {
        Workspace {
            name: name.to_string(),
            mounts: Vec::new(),
        }
    }

    #[test]
    fn runtime_project_is_scoped_by_workspace_and_mode() {
        let workspace = workspace("Kubernetes API");

        assert_eq!(
            runtime_project_name(&workspace, true),
            "agent-box-kubernetes-api-docker"
        );
        assert_eq!(
            runtime_project_name(&workspace, false),
            "agent-box-kubernetes-api-plain"
        );
    }

    #[test]
    fn compose_files_are_consolidated() {
        assert_eq!(
            Agent::Claude.compose_files(false),
            vec!["docker-compose.yml"]
        );
        assert_eq!(
            Agent::Codex.compose_files(true),
            vec!["docker-compose.yml", "docker-compose.docker.yml"]
        );
    }

    #[test]
    fn rebuild_flag_is_forwarded_to_shared_proxy() {
        let rebuild_args = shared_proxy_command(Path::new("/sandbox"), true)
            .get_args()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        let reuse_args = shared_proxy_command(Path::new("/sandbox"), false)
            .get_args()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        assert!(rebuild_args.iter().any(|argument| argument == "--build"));
        assert!(!reuse_args.iter().any(|argument| argument == "--build"));
    }

    #[test]
    fn config_rejects_colliding_runtime_names() {
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
    fn mount_targets_are_stable_inside_agent_and_dind() {
        let mounts = vec![Mount {
            name: "api".to_string(),
            host: PathBuf::from("/host/workspace/api"),
        }];
        let mut agent = String::new();
        let mut daemon = String::new();

        write_service_mounts(&mut agent, "codex-agent", &mounts);
        write_service_mounts(&mut daemon, "docker-daemon", &mounts);

        assert!(agent.contains("target: /home/devops/project/api"));
        assert!(daemon.contains("target: /home/devops/project/api"));
    }
}

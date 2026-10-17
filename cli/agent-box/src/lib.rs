mod compose;
mod config;
mod runtime;

use compose::{
    container_context, runtime_project_name, write_compose_override, write_forwarder_override,
};
use config::{load_config, Defaults, Mount, Workspace};
use dialoguer::{theme::ColorfulTheme, FuzzySelect};
use std::env;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
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

    fn instruction_filename(self) -> &'static str {
        match self {
            Agent::Claude => "CLAUDE.md",
            Agent::Codex => "AGENTS.md",
        }
    }

    fn instruction_container_path(self) -> &'static str {
        match self {
            Agent::Claude => "/home/devops/.claude/CLAUDE.md",
            Agent::Codex => "/home/devops/.codex/AGENTS.md",
        }
    }

    fn default_home(self) -> Option<PathBuf> {
        let home = env::var_os("HOME").map(PathBuf::from)?;
        Some(match self {
            Agent::Claude => home.join(".ai-sandbox").join("claude"),
            Agent::Codex => home.join(".ai-sandbox").join("codex"),
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
    let launch_dir = env::current_dir()
        .map_err(|error| format!("cannot determine current directory: {error}"))?
        .canonicalize()
        .map_err(|error| format!("cannot resolve current directory: {error}"))?;
    let runtime_paths = runtime::prepare()?;
    let mut config = load_config()?;
    config
        .workspaces
        .push(current_directory_workspace(&launch_dir));
    ensure_agent_home(agent, &config.defaults)?;
    let docker_enabled = config.defaults.docker && !cli_no_docker;
    let workspace = choose_workspace(&config.workspaces)?;
    let mount = choose_mount(workspace)?;
    let project_name = runtime_project_name(workspace, docker_enabled);
    let workdir = container_context(&workspace.name, &mount.name);
    if let Some(port_range) = workspace.docker_ports.filter(|_| docker_enabled) {
        println!(
            "Docker ports published to host for '{}': {}",
            workspace.name, port_range
        );
    }
    let override_path = write_compose_override(
        agent,
        &runtime_paths.assets,
        &runtime_paths.generated,
        &config.defaults,
        workspace,
        docker_enabled,
    )?;
    let forwarder_override =
        write_forwarder_override(&runtime_paths.generated, &config.workspaces)?;

    start_shared_services(
        &runtime_paths.assets,
        forwarder_override.as_deref(),
        rebuild,
    )?;
    start_stack(
        agent,
        &runtime_paths.assets,
        &override_path,
        &config.defaults,
        &project_name,
        docker_enabled,
        rebuild,
    )?;
    exec_agent(
        agent,
        &runtime_paths.assets,
        &override_path,
        &config.defaults,
        &project_name,
        &workdir,
        docker_enabled,
    )
}

fn print_help(agent: Agent) {
    println!("Usage: {} [--no-docker] [--build]", agent_box_name(agent));
    println!();
    println!("Reads workspace definitions from ~/.config/ai-sandbox/box.toml by default.");
    println!("The current directory is always available as the last workspace option.");
    println!("Runtime Compose and build assets are embedded in the executable.");
    println!("By default, starts existing images with docker compose up -d.");
    println!("Pass --build to rebuild images before opening the agent.");
    println!("Set AI_SANDBOX_DIR only to use runtime assets from a development checkout.");
}

fn agent_box_name(agent: Agent) -> &'static str {
    match agent {
        Agent::Claude => "claude-box",
        Agent::Codex => "codex-box",
    }
}

fn current_directory_workspace(path: &Path) -> Workspace {
    let mount_name = path
        .file_name()
        .filter(|name| !name.is_empty())
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "project".to_string());
    Workspace {
        name: "current".to_string(),
        mounts: vec![Mount {
            name: mount_name,
            host: path.to_path_buf(),
        }],
        docker_ports: None,
        display_name: Some("Current directory".to_string()),
        runtime_name: Some(format!("current-{:016x}", stable_path_hash(path))),
    }
}

fn stable_path_hash(path: &Path) -> u64 {
    path.to_string_lossy()
        .as_bytes()
        .iter()
        .fold(0xcbf29ce484222325, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        })
}

fn agent_home(agent: Agent, defaults: &Defaults) -> Option<PathBuf> {
    if let Some(path) = env::var_os(agent.home_env()) {
        return Some(PathBuf::from(path));
    }
    configured_agent_home(agent, defaults).or_else(|| agent.default_home())
}

fn ensure_agent_home(agent: Agent, defaults: &Defaults) -> Result<(), String> {
    let path = agent_home(agent, defaults)
        .ok_or_else(|| format!("cannot determine {}; HOME is not set", agent.home_env()))?;
    let existed = path.exists();
    fs::create_dir_all(&path)
        .map_err(|error| format!("cannot create {}: {error}", path.display()))?;

    #[cfg(unix)]
    {
        let uses_isolated_default = configured_agent_home(agent, defaults).is_none();
        if uses_isolated_default {
            if let Some(root) = path.parent() {
                fs::set_permissions(root, fs::Permissions::from_mode(0o700))
                    .map_err(|error| format!("cannot secure {}: {error}", root.display()))?;
            }
        }
        if uses_isolated_default || !existed {
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700))
                .map_err(|error| format!("cannot secure {}: {error}", path.display()))?;
        }
    }

    Ok(())
}

fn configured_agent_home(agent: Agent, defaults: &Defaults) -> Option<PathBuf> {
    if let Some(path) = env::var_os(agent.home_env()) {
        return Some(PathBuf::from(path));
    }
    match agent {
        Agent::Claude => defaults.claude_home.clone(),
        Agent::Codex => defaults.codex_home.clone(),
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

fn start_shared_services(
    sandbox_dir: &Path,
    forwarder_override: Option<&Path>,
    rebuild: bool,
) -> Result<(), String> {
    if forwarder_override.is_none() {
        run_command(remove_shared_forwarder_command(sandbox_dir))?;
    }
    run_command(shared_services_command(
        sandbox_dir,
        forwarder_override,
        rebuild,
    ))
}

fn shared_services_command(
    sandbox_dir: &Path,
    forwarder_override: Option<&Path>,
    rebuild: bool,
) -> Command {
    let mut command = Command::new("docker");
    command
        .current_dir(sandbox_dir)
        .arg("compose")
        .arg("--project-name")
        .arg("ai-sandbox")
        .arg("-f")
        .arg("docker-compose.yml");
    if let Some(override_path) = forwarder_override {
        command
            .arg("-f")
            .arg(override_path)
            .arg("--profile")
            .arg("ports");
    }
    command.arg("up").arg("-d").arg("--wait");
    if rebuild {
        command.arg("--build");
    }
    command.arg("proxy");
    if forwarder_override.is_some() {
        command.arg("host-dind-forwarder");
    }
    command
}

fn remove_shared_forwarder_command(sandbox_dir: &Path) -> Command {
    let mut command = Command::new("docker");
    command
        .current_dir(sandbox_dir)
        .arg("compose")
        .arg("--project-name")
        .arg("ai-sandbox")
        .arg("-f")
        .arg("docker-compose.yml")
        .arg("--profile")
        .arg("ports")
        .arg("rm")
        .arg("-sf")
        .arg("host-dind-forwarder");
    command
}

fn choose_workspace(workspaces: &[Workspace]) -> Result<&Workspace, String> {
    if workspaces.len() == 1 {
        return Ok(&workspaces[0]);
    }

    let options = workspaces
        .iter()
        .map(|workspace| {
            if let Some(display_name) = workspace.display_name.as_deref() {
                return format!(
                    "{:<18} {}",
                    display_name,
                    workspace.mounts[0].host.display()
                );
            }
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
    let workspace_name = workspace.display_name.as_deref().unwrap_or(&workspace.name);
    let selected = choose_from_menu(&format!("{workspace_name} context"), &options)?;
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
    set_agent_home_env(agent, defaults, &mut command);
    set_runtime_env(agent, project_name, docker_enabled, &mut command);
    run_command(command)
}

fn exec_agent(
    agent: Agent,
    sandbox_dir: &Path,
    override_path: &Path,
    defaults: &Defaults,
    project_name: &str,
    workdir: &str,
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
        .arg(workdir)
        .arg(agent.service())
        .arg(agent.entrypoint())
        .arg(agent.command());
    set_agent_home_env(agent, defaults, &mut command);
    set_runtime_env(agent, project_name, docker_enabled, &mut command);
    run_command(command)
}

fn set_agent_home_env(agent: Agent, defaults: &Defaults, command: &mut Command) {
    if let Some(path) = agent_home(agent, defaults) {
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
    fn rebuild_flag_is_forwarded_to_shared_services() {
        let rebuild_args = shared_services_command(Path::new("/sandbox"), None, true)
            .get_args()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        let reuse_args = shared_services_command(Path::new("/sandbox"), None, false)
            .get_args()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        assert!(rebuild_args.iter().any(|argument| argument == "--build"));
        assert!(!reuse_args.iter().any(|argument| argument == "--build"));
    }

    #[test]
    fn shared_forwarder_uses_generated_override_and_ports_profile() {
        let args = shared_services_command(
            Path::new("/sandbox"),
            Some(Path::new("/tmp/forwarder.yml")),
            false,
        )
        .get_args()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect::<Vec<_>>();

        assert!(args
            .windows(2)
            .any(|args| args == ["-f", "/tmp/forwarder.yml"]));
        assert!(args.windows(2).any(|args| args == ["--profile", "ports"]));
        assert!(args
            .iter()
            .any(|argument| argument == "host-dind-forwarder"));
    }

    #[test]
    fn current_directories_get_distinct_stable_runtimes() {
        let first = current_directory_workspace(Path::new("/work/team/api"));
        let repeated = current_directory_workspace(Path::new("/work/team/api"));
        let second = current_directory_workspace(Path::new("/other/team/api"));

        assert_eq!(first.runtime_name, repeated.runtime_name);
        assert_ne!(first.runtime_name, second.runtime_name);
        assert_eq!(first.mounts[0].name, "api");
        assert_eq!(first.mounts[0].host, Path::new("/work/team/api"));
        assert_eq!(first.display_name.as_deref(), Some("Current directory"));
    }
}

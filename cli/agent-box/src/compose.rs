use crate::config::{Defaults, Mount, PortRange, Workspace};
use crate::{agent_home, sanitize_name, Agent};
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn write_compose_override(
    agent: Agent,
    sandbox_dir: &Path,
    output_dir: &Path,
    defaults: &Defaults,
    workspace: &Workspace,
    docker_enabled: bool,
) -> Result<PathBuf, String> {
    fs::create_dir_all(output_dir).map_err(|error| error.to_string())?;

    let output_path = output_dir.join(format!(
        "{}-{}.yml",
        agent.command(),
        workspace_runtime_name(workspace)
    ));
    let instruction_path = if docker_enabled {
        workspace
            .docker_ports
            .map(|port_range| {
                write_runtime_instructions(
                    agent,
                    sandbox_dir,
                    output_dir,
                    defaults,
                    workspace,
                    port_range,
                )
            })
            .transpose()?
    } else {
        None
    };

    let mut contents = String::from("services:\n");
    write_service_mounts(
        &mut contents,
        agent.service(),
        &workspace.name,
        &workspace.mounts,
    );
    if let Some(path) = instruction_path.as_deref() {
        write_instruction_mount(&mut contents, agent, path);
    }
    contents.push_str("    command: [\"sleep\", \"infinity\"]\n");
    if let Some(port_range) = workspace.docker_ports.filter(|_| docker_enabled) {
        write_port_environment(&mut contents, port_range);
    }
    if docker_enabled {
        contents.push_str(
            "    depends_on: !override\n      docker-daemon:\n        condition: service_healthy\n",
        );
        write_service_mounts(
            &mut contents,
            "docker-daemon",
            &workspace.name,
            &workspace.mounts,
        );
        if let Some(port_range) = workspace.docker_ports {
            write_port_environment(&mut contents, port_range);
            contents.push_str(&format!(
                "    networks: !override\n      agent_net:\n        aliases:\n          - {}\n",
                dind_network_alias(workspace)
            ));
        }
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

pub(crate) fn write_forwarder_override(
    output_dir: &Path,
    workspaces: &[Workspace],
) -> Result<Option<PathBuf>, String> {
    let forwarded_workspaces = workspaces
        .iter()
        .filter_map(|workspace| workspace.docker_ports.map(|ports| (workspace, ports)))
        .collect::<Vec<_>>();
    if forwarded_workspaces.is_empty() {
        return Ok(None);
    }

    fs::create_dir_all(output_dir).map_err(|error| error.to_string())?;

    let haproxy_config = render_haproxy_config(&forwarded_workspaces);
    let config_hash = stable_hash(&haproxy_config);
    let haproxy_path = output_dir.join(format!("haproxy-{config_hash:016x}.cfg"));
    fs::write(&haproxy_path, haproxy_config).map_err(|error| error.to_string())?;

    let override_path = output_dir.join("host-dind-forwarder.yml");
    let contents = render_forwarder_override(&forwarded_workspaces, &haproxy_path, config_hash);
    fs::write(&override_path, contents).map_err(|error| error.to_string())?;

    Ok(Some(override_path))
}

fn render_forwarder_override(
    workspaces: &[(&Workspace, PortRange)],
    haproxy_path: &Path,
    config_hash: u64,
) -> String {
    let mut contents = String::from("services:\n  host-dind-forwarder:\n    ports:\n");
    for (_, port_range) in workspaces {
        contents.push_str(&format!(
            "      - \"127.0.0.1:{port_range}:{port_range}\"\n"
        ));
    }
    contents.push_str(&format!(
        "    volumes:\n      - type: bind\n        source: {}\n        target: /usr/local/etc/haproxy/haproxy.cfg\n        read_only: true\n    labels:\n      ai-sandbox.forwarder-config: \"{config_hash:016x}\"\n",
        yaml_path(haproxy_path)
    ));
    contents
}

pub(crate) fn runtime_project_name(workspace: &Workspace, docker_enabled: bool) -> String {
    let workspace_name = workspace_runtime_name(workspace);
    let mode = if docker_enabled { "docker" } else { "plain" };
    format!("agent-box-{workspace_name}-{mode}")
}

pub(crate) fn container_context(workspace_name: &str, mount_name: &str) -> String {
    format!(
        "/home/devops/project/{}/{}",
        sanitize_name(workspace_name).to_ascii_lowercase(),
        yaml_segment(mount_name)
    )
}

fn write_instruction_mount(contents: &mut String, agent: Agent, path: &Path) {
    contents.push_str(&format!(
        "      - type: bind\n        source: {}\n        target: {}\n        read_only: true\n",
        yaml_path(path),
        agent.instruction_container_path()
    ));
}

fn dind_network_alias(workspace: &Workspace) -> String {
    format!("dind-{}", workspace_runtime_name(workspace))
}

fn workspace_runtime_name(workspace: &Workspace) -> String {
    sanitize_name(workspace.runtime_name.as_deref().unwrap_or(&workspace.name)).to_ascii_lowercase()
}

fn write_port_environment(contents: &mut String, port_range: PortRange) {
    contents.push_str(&format!(
        "    environment:\n      SANDBOX_DOCKER_PORT_RANGE: \"{port_range}\"\n"
    ));
}

fn render_haproxy_config(workspaces: &[(&Workspace, PortRange)]) -> String {
    let mut contents = String::from(
        "global\n    log stdout format raw local0\n\ndefaults\n    log global\n    mode tcp\n    timeout connect 5s\n    timeout client 1h\n    timeout server 1h\n\nresolvers docker\n    nameserver dns 127.0.0.11:53\n    resolve_retries 3\n    timeout resolve 1s\n    timeout retry 1s\n    hold other 10s\n    hold refused 10s\n    hold nx 10s\n    hold timeout 10s\n    hold valid 10s\n    hold obsolete 10s\n",
    );

    for (workspace, port_range) in workspaces {
        let alias = dind_network_alias(workspace);
        let listener_name = sanitize_name(&workspace.name).to_ascii_lowercase();
        for port in port_range.start..=port_range.end {
            contents.push_str(&format!(
                "\nlisten {listener_name}-{port}\n    bind :{port}\n    server dind {alias}:{port} resolvers docker init-addr libc,none\n"
            ));
        }
    }

    contents
}

fn stable_hash(value: &str) -> u64 {
    value
        .as_bytes()
        .iter()
        .fold(0xcbf29ce484222325, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        })
}

fn write_runtime_instructions(
    agent: Agent,
    sandbox_dir: &Path,
    output_dir: &Path,
    defaults: &Defaults,
    workspace: &Workspace,
    port_range: PortRange,
) -> Result<PathBuf, String> {
    let instruction_dir = output_dir.join("instructions");
    fs::create_dir_all(&instruction_dir).map_err(|error| error.to_string())?;
    let instruction_path = instruction_dir.join(format!(
        "{}-{}-{}",
        agent.command(),
        workspace_runtime_name(workspace),
        agent.instruction_filename()
    ));

    let existing_instructions = agent_home(agent, defaults)
        .map(|home| home.join(agent.instruction_filename()))
        .filter(|path| path.is_file())
        .map(|path| fs::read_to_string(&path).map_err(|error| error.to_string()))
        .transpose()?
        .unwrap_or_default();
    let template_path = sandbox_dir.join("instructions").join("docker-ports.md");
    let template = fs::read_to_string(&template_path)
        .map_err(|error| format!("cannot read {}: {error}", template_path.display()))?;
    let contents = combined_runtime_instructions(&existing_instructions, &template, port_range);
    fs::write(&instruction_path, contents).map_err(|error| error.to_string())?;
    Ok(instruction_path)
}

fn combined_runtime_instructions(existing: &str, template: &str, port_range: PortRange) -> String {
    let mut contents = existing.trim_end().to_string();
    if !contents.is_empty() {
        contents.push_str("\n\n");
    }
    let rendered = template
        .replace("{{docker_port_range}}", &port_range.to_string())
        .replace("{{example_port}}", &port_range.start.to_string());
    contents.push_str(rendered.trim());
    contents.push('\n');
    contents
}

fn write_service_mounts(
    contents: &mut String,
    service: &str,
    workspace_name: &str,
    mounts: &[Mount],
) {
    contents.push_str(&format!("  {service}:\n"));
    contents.push_str("    volumes:\n");
    for mount in mounts {
        contents.push_str(&format!(
            "      - type: bind\n        source: {}\n        target: {}\n",
            yaml_path(&mount.host),
            container_context(workspace_name, &mount.name)
        ));
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace(name: &str) -> Workspace {
        Workspace {
            name: name.to_string(),
            mounts: Vec::new(),
            docker_ports: None,
            display_name: None,
            runtime_name: None,
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
    fn explicit_runtime_name_disambiguates_dynamic_workspace() {
        let mut workspace = workspace("current");
        workspace.runtime_name = Some("current-deadbeef".to_string());

        assert_eq!(
            runtime_project_name(&workspace, true),
            "agent-box-current-deadbeef-docker"
        );
        assert_eq!(dind_network_alias(&workspace), "dind-current-deadbeef");
    }

    #[test]
    fn mount_targets_are_stable_inside_agent_and_dind() {
        let mounts = vec![Mount {
            name: "api".to_string(),
            host: PathBuf::from("/host/workspace/api"),
        }];
        let mut agent = String::new();
        let mut daemon = String::new();

        write_service_mounts(&mut agent, "codex-agent", "Platform", &mounts);
        write_service_mounts(&mut daemon, "docker-daemon", "Platform", &mounts);

        assert!(agent.contains("target: /home/devops/project/platform/api"));
        assert!(daemon.contains("target: /home/devops/project/platform/api"));
    }

    #[test]
    fn selected_context_is_scoped_by_workspace() {
        assert_eq!(
            container_context("Kubernetes API", "backend"),
            "/home/devops/project/kubernetes-api/backend"
        );
    }

    #[test]
    fn forwarder_binds_workspace_range_to_loopback() {
        let mut workspace = workspace("Platform");
        workspace.docker_ports = Some(PortRange {
            start: 18000,
            end: 18099,
        });

        let contents = render_forwarder_override(
            &[(&workspace, workspace.docker_ports.unwrap())],
            Path::new("/tmp/haproxy.cfg"),
            42,
        );

        assert!(contents.contains("host-dind-forwarder:"));
        assert!(contents.contains("127.0.0.1:18000-18099:18000-18099"));
        assert!(contents.contains("source: \"/tmp/haproxy.cfg\""));
        assert!(contents.contains("ai-sandbox.forwarder-config: \"000000000000002a\""));
    }

    #[test]
    fn haproxy_forwards_each_port_to_workspace_dind_alias() {
        let mut workspace = workspace("Platform API");
        let port_range = PortRange {
            start: 18000,
            end: 18001,
        };
        workspace.docker_ports = Some(port_range);

        let contents = render_haproxy_config(&[(&workspace, port_range)]);

        assert!(contents.contains("listen platform-api-18000"));
        assert!(contents.contains("server dind dind-platform-api:18000"));
        assert!(contents.contains("listen platform-api-18001"));
        assert!(contents.contains("server dind dind-platform-api:18001"));
    }

    #[test]
    fn runtime_instructions_preserve_user_instructions() {
        let instructions = combined_runtime_instructions(
            "# Existing instructions\n\n- Keep this rule.\n",
            "Use `$SANDBOX_DOCKER_PORT_RANGE={{docker_port_range}}`. Example: `docker run -p {{example_port}}:80 nginx`.",
            PortRange {
                start: 18000,
                end: 18099,
            },
        );

        assert!(instructions.starts_with("# Existing instructions"));
        assert!(instructions.contains("$SANDBOX_DOCKER_PORT_RANGE"));
        assert!(instructions.contains("docker run -p 18000:80 nginx"));
    }
}

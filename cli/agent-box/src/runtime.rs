use std::env;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

struct Asset {
    path: &'static str,
    contents: &'static [u8],
}

const ASSETS: &[Asset] = &[
    Asset {
        path: ".env",
        contents: include_bytes!("../../../.env"),
    },
    Asset {
        path: "docker-compose.yml",
        contents: include_bytes!("../../../docker-compose.yml"),
    },
    Asset {
        path: "docker-compose.docker.yml",
        contents: include_bytes!("../../../docker-compose.docker.yml"),
    },
    Asset {
        path: "Dockerfile.agent",
        contents: include_bytes!("../../../Dockerfile.agent"),
    },
    Asset {
        path: "instructions/docker-ports.md",
        contents: include_bytes!("../../../instructions/docker-ports.md"),
    },
    Asset {
        path: "instructions/proxy-allowlist.md",
        contents: include_bytes!("../../../instructions/proxy-allowlist.md"),
    },
    Asset {
        path: "proxy/Dockerfile",
        contents: include_bytes!("../../../proxy/Dockerfile"),
    },
    Asset {
        path: "proxy/allowed-domains.txt",
        contents: include_bytes!("../../../proxy/allowed-domains.txt"),
    },
    Asset {
        path: "proxy/squid.conf",
        contents: include_bytes!("../../../proxy/squid.conf"),
    },
    Asset {
        path: "proxy/start-squid.sh",
        contents: include_bytes!("../../../proxy/start-squid.sh"),
    },
    Asset {
        path: "scripts/agent-entrypoint-lib.sh",
        contents: include_bytes!("../../../scripts/agent-entrypoint-lib.sh"),
    },
    Asset {
        path: "scripts/claude-entrypoint.sh",
        contents: include_bytes!("../../../scripts/claude-entrypoint.sh"),
    },
    Asset {
        path: "scripts/codex-entrypoint.sh",
        contents: include_bytes!("../../../scripts/codex-entrypoint.sh"),
    },
];

pub(crate) struct RuntimePaths {
    pub(crate) assets: PathBuf,
    pub(crate) generated: PathBuf,
}

pub(crate) fn prepare() -> Result<RuntimePaths, String> {
    let cache_root = cache_root()?;
    let generated = cache_root.join("ai-sandbox").join("generated");
    fs::create_dir_all(&generated)
        .map_err(|error| format!("cannot create {}: {error}", generated.display()))?;
    #[cfg(unix)]
    fs::set_permissions(&generated, fs::Permissions::from_mode(0o700))
        .map_err(|error| format!("cannot secure {}: {error}", generated.display()))?;

    if let Some(path) = env::var_os("AI_SANDBOX_DIR") {
        let assets = PathBuf::from(path);
        validate_assets(&assets)?;
        return Ok(RuntimePaths { assets, generated });
    }

    let assets = cache_root.join("ai-sandbox").join("runtime").join(format!(
        "{}-{:016x}",
        env!("CARGO_PKG_VERSION"),
        assets_hash()
    ));
    materialize_assets(&assets)?;
    Ok(RuntimePaths { assets, generated })
}

fn cache_root() -> Result<PathBuf, String> {
    #[cfg(target_os = "macos")]
    {
        home_dir().map(|home| home.join("Library").join("Caches"))
    }

    #[cfg(not(target_os = "macos"))]
    {
        if let Some(path) = env::var_os("XDG_CACHE_HOME") {
            let path = PathBuf::from(path);
            if path.is_absolute() {
                return Ok(path);
            }
        }
        home_dir().map(|home| home.join(".cache"))
    }
}

fn home_dir() -> Result<PathBuf, String> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "cannot determine runtime cache directory; HOME is not set".to_string())
}

fn materialize_assets(root: &Path) -> Result<(), String> {
    for asset in ASSETS {
        let path = root.join(asset.path);
        if fs::read(&path).ok().as_deref() == Some(asset.contents) {
            continue;
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
        }
        let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
        fs::write(&temporary, asset.contents)
            .map_err(|error| format!("cannot write {}: {error}", temporary.display()))?;
        fs::rename(&temporary, &path)
            .map_err(|error| format!("cannot install {}: {error}", path.display()))?;
    }
    Ok(())
}

fn validate_assets(root: &Path) -> Result<(), String> {
    for asset in ASSETS {
        let path = root.join(asset.path);
        if !path.is_file() {
            return Err(format!(
                "AI_SANDBOX_DIR is missing required asset {}",
                path.display()
            ));
        }
    }
    Ok(())
}

fn assets_hash() -> u64 {
    ASSETS.iter().fold(0xcbf29ce484222325, |hash, asset| {
        let hash = hash_bytes(hash, asset.path.as_bytes());
        hash_bytes(hash, asset.contents)
    })
}

fn hash_bytes(initial: u64, bytes: &[u8]) -> u64 {
    bytes.iter().fold(initial, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn embedded_runtime_contains_required_build_assets() {
        let paths = ASSETS.iter().map(|asset| asset.path).collect::<Vec<_>>();

        assert!(paths.contains(&"docker-compose.yml"));
        assert!(paths.contains(&"docker-compose.docker.yml"));
        assert!(paths.contains(&"Dockerfile.agent"));
        assert!(paths.contains(&"instructions/proxy-allowlist.md"));
        assert!(paths.contains(&"proxy/Dockerfile"));
        assert!(paths.contains(&"scripts/claude-entrypoint.sh"));
        assert!(paths.contains(&"scripts/codex-entrypoint.sh"));
    }

    #[test]
    fn embedded_runtime_hash_is_stable() {
        assert_eq!(assets_hash(), assets_hash());
        assert_ne!(assets_hash(), 0);
    }

    #[test]
    fn materializes_embedded_runtime_without_repository_access() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = env::temp_dir().join(format!(
            "ai-sandbox-runtime-test-{}-{unique}",
            std::process::id()
        ));

        materialize_assets(&root).unwrap();

        assert_eq!(
            fs::read(root.join("docker-compose.yml")).unwrap(),
            include_bytes!("../../../docker-compose.yml")
        );
        assert_eq!(
            fs::read(root.join("proxy/squid.conf")).unwrap(),
            include_bytes!("../../../proxy/squid.conf")
        );

        fs::remove_dir_all(root).unwrap();
    }
}

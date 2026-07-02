use std::env;
use std::fs;
use std::net::IpAddr;
use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct Config {
    pub api_url: String,
    pub api_key: String,
}

#[derive(Debug, Default, Deserialize)]
struct RuntimeConfig {
    #[serde(rename = "external-controller")]
    external_controller: Option<String>,
    #[serde(default)]
    secret: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InstallEnv {
    pub base_dir: PathBuf,
    pub service_name: Option<String>,
    pub kernel_name: Option<String>,
}

impl InstallEnv {
    fn new(base_dir: PathBuf) -> Self {
        Self {
            base_dir,
            service_name: None,
            kernel_name: None,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let (api_url, api_key) = Self::resolve();
        Self { api_url, api_key }
    }

    fn resolve() -> (String, String) {
        if let Some((url, key)) = Self::read_config_file() {
            return (url, key);
        }

        for runtime in Self::runtime_paths() {
            if let Ok(content) = fs::read_to_string(&runtime) {
                if let Some((url, key)) = runtime_config_from_yaml(&content) {
                    return (url, key);
                }
            }
        }

        ("http://127.0.0.1:9090".into(), String::new())
    }

    fn read_config_file() -> Option<(String, String)> {
        let config_path = Self::config_path()?;
        let content = fs::read_to_string(&config_path).ok()?;
        let mut url = None;
        let mut key = None;

        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('#') || line.is_empty() {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                match k.trim() {
                    "API_URL" => url = Some(v.trim().to_string()),
                    "API_KEY" => key = Some(v.trim().to_string()),
                    _ => {}
                }
            }
        }
        Some((url?, key.unwrap_or_default()))
    }

    fn config_path() -> Option<PathBuf> {
        let home = env::var("HOME").ok()?;
        let path = PathBuf::from(&home)
            .join(".config")
            .join("clash-tui")
            .join("config.env");
        if path.exists() {
            return Some(path);
        }
        let local = PathBuf::from("config.env");
        if local.exists() {
            return Some(local);
        }
        None
    }

    fn runtime_paths() -> Vec<PathBuf> {
        resource_paths()
            .into_iter()
            .map(|p| p.join("runtime.yaml"))
            .collect()
    }
}

pub(crate) fn active_install_env() -> Option<InstallEnv> {
    active_install_env_from_candidates(install_env_candidates())
}

fn active_install_env_from_candidates(candidates: Vec<InstallEnv>) -> Option<InstallEnv> {
    for candidate in &candidates {
        if candidate.base_dir.join("resources").exists()
            || candidate.base_dir.join("install-state.json").exists()
        {
            return Some(candidate.clone());
        }
    }
    candidates.into_iter().next()
}

pub(crate) fn resource_paths() -> Vec<PathBuf> {
    install_env_candidates()
        .into_iter()
        .map(|env| env.base_dir.join("resources"))
        .collect()
}

fn install_env_candidates() -> Vec<InstallEnv> {
    let home = env::var("HOME").ok().map(PathBuf::from);
    let process_env = install_env_from_process(home.as_deref());
    let user_marker_base = home.as_deref().and_then(|home| {
        read_install_marker_file(
            &home.join(".config").join("clashctl").join("install.env"),
            Some(home),
        )
    });
    let system_marker_base =
        read_install_marker_file(Path::new("/etc/clashctl/install.env"), home.as_deref());
    install_env_candidates_from_sources(
        process_env,
        user_marker_base,
        system_marker_base,
        home.as_deref(),
    )
}

fn install_env_from_process(home: Option<&Path>) -> Option<InstallEnv> {
    let base = env::var("CLASH_BASE_DIR").ok()?;
    let base = base.trim();
    if base.is_empty() {
        return None;
    }
    Some(InstallEnv {
        base_dir: expand_home_with_optional(base, home),
        service_name: non_empty_env("SERVICE_NAME").or_else(|| non_empty_env("CLASH_SERVICE_NAME")),
        kernel_name: non_empty_env("KERNEL_NAME"),
    })
}

fn non_empty_env(key: &str) -> Option<String> {
    env::var(key).ok().and_then(|v| non_empty_string(&v))
}

fn install_env_candidates_from_sources(
    process_env: Option<InstallEnv>,
    user_marker: Option<InstallEnv>,
    system_marker: Option<InstallEnv>,
    home: Option<&Path>,
) -> Vec<InstallEnv> {
    let mut paths = Vec::new();
    if let Some(env) = process_env {
        push_unique_install_env(&mut paths, env);
    }
    if let Some(env) = user_marker {
        push_unique_install_env(&mut paths, env);
    }
    if let Some(env) = system_marker {
        push_unique_install_env(&mut paths, env);
    }
    if let Some(home) = home {
        for base in &["clashctl", ".clashctl"] {
            push_unique_install_env(&mut paths, InstallEnv::new(home.join(base)));
        }
    }
    paths
}

fn push_unique_install_env(paths: &mut Vec<InstallEnv>, env: InstallEnv) {
    if let Some(existing) = paths.iter_mut().find(|p| p.base_dir == env.base_dir) {
        if existing.service_name.is_none() {
            existing.service_name = env.service_name;
        }
        if existing.kernel_name.is_none() {
            existing.kernel_name = env.kernel_name;
        }
    } else {
        paths.push(env);
    }
}

fn read_install_marker_file(path: &Path, home: Option<&Path>) -> Option<InstallEnv> {
    let content = fs::read_to_string(path).ok()?;
    let mut base_dir = None;
    let mut service_name = None;
    let mut kernel_name = None;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once('=') {
            let key = key.trim();
            let value = value.trim().trim_matches('"').trim_matches('\'');
            match key {
                "CLASH_BASE_DIR" => {
                    if !value.is_empty() {
                        base_dir = Some(expand_home_with_optional(value, home));
                    }
                }
                "SERVICE_NAME" | "CLASH_SERVICE_NAME" => {
                    service_name = non_empty_string(value);
                }
                "KERNEL_NAME" => {
                    kernel_name = non_empty_string(value);
                }
                _ => {}
            }
        }
    }
    Some(InstallEnv {
        base_dir: base_dir?,
        service_name,
        kernel_name,
    })
}

fn non_empty_string(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

#[cfg(test)]
fn expand_home(path: &str, home: &Path) -> PathBuf {
    expand_home_with_optional(path, Some(home))
}

fn expand_home_with_optional(path: &str, home: Option<&Path>) -> PathBuf {
    if path == "~" {
        return home
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from(path));
    }
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = home {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

fn runtime_config_from_yaml(content: &str) -> Option<(String, String)> {
    let runtime: RuntimeConfig = serde_yaml::from_str(content).ok()?;
    let url = api_url_from_controller(runtime.external_controller.as_deref());
    Some((url, runtime.secret))
}

fn api_url_from_controller(controller: Option<&str>) -> String {
    let (host, port) = split_controller(controller.unwrap_or_default());
    format!("http://{}", join_host_port(&dial_host(&host), &port))
}

fn split_controller(controller: &str) -> (String, String) {
    let mut value = controller.trim().trim_matches('"').trim_matches('\'');
    if let Some(rest) = value.strip_prefix("http://") {
        value = rest;
    } else if let Some(rest) = value.strip_prefix("https://") {
        value = rest;
    }

    if let Some(rest) = value.strip_prefix('[') {
        if let Some(close) = rest.find(']') {
            let host = &rest[..close];
            let after = rest[close + 1..].trim_start_matches(':');
            return (host.to_string(), default_port(after));
        }
    }

    if value.matches(':').count() == 1 {
        if let Some((host, port)) = value.rsplit_once(':') {
            return (host.to_string(), default_port(port));
        }
    }

    (value.trim_matches(['[', ']']).to_string(), "9090".into())
}

fn default_port(port: &str) -> String {
    let port = port.trim();
    if port.is_empty() {
        "9090".into()
    } else {
        port.to_string()
    }
}

fn dial_host(host: &str) -> String {
    let host = host.trim().trim_matches(['[', ']']);
    if host.is_empty() || host == "0.0.0.0" {
        return "127.0.0.1".into();
    }
    if host == "::" {
        return "::1".into();
    }
    if let Ok(ip) = host.parse::<IpAddr>() {
        if ip.is_unspecified() {
            return if ip.is_ipv6() {
                "::1".into()
            } else {
                "127.0.0.1".into()
            };
        }
    }
    host.to_string()
}

fn join_host_port(host: &str, port: &str) -> String {
    if host.contains(':') && !host.starts_with('[') {
        format!("[{}]:{}", host, port)
    } else {
        format!("{}:{}", host, port)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        active_install_env_from_candidates, api_url_from_controller, expand_home,
        install_env_candidates_from_sources, read_install_marker_file, runtime_config_from_yaml,
        InstallEnv,
    };
    use std::fs;
    use std::path::{Path, PathBuf};

    fn install_env(
        base_dir: impl Into<PathBuf>,
        service_name: Option<&str>,
        kernel_name: Option<&str>,
    ) -> InstallEnv {
        InstallEnv {
            base_dir: base_dir.into(),
            service_name: service_name.map(str::to_string),
            kernel_name: kernel_name.map(str::to_string),
        }
    }

    fn base_dirs(candidates: &[InstallEnv]) -> Vec<PathBuf> {
        candidates
            .iter()
            .map(|candidate| candidate.base_dir.clone())
            .collect()
    }

    #[test]
    fn controller_to_api_url_handles_ipv6_loopback() {
        assert_eq!(
            api_url_from_controller(Some("[::1]:9090")),
            "http://[::1]:9090"
        );
    }

    #[test]
    fn controller_to_api_url_dials_loopback_for_unspecified_listeners() {
        assert_eq!(
            api_url_from_controller(Some("0.0.0.0:9090")),
            "http://127.0.0.1:9090"
        );
        assert_eq!(
            api_url_from_controller(Some("[::]:9090")),
            "http://[::1]:9090"
        );
    }

    #[test]
    fn controller_to_api_url_uses_default_port() {
        assert_eq!(
            api_url_from_controller(Some("localhost")),
            "http://localhost:9090"
        );
        assert_eq!(api_url_from_controller(None), "http://127.0.0.1:9090");
    }

    #[test]
    fn runtime_yaml_is_parsed_structurally() {
        let parsed = runtime_config_from_yaml(
            r#"
mixed-port: 7890
external-controller: "[::1]:9090"
secret: "abc"
"#,
        )
        .expect("runtime config should parse");
        assert_eq!(parsed.0, "http://[::1]:9090");
        assert_eq!(parsed.1, "abc");
    }

    #[test]
    fn home_paths_expand_like_clashctl() {
        assert_eq!(
            expand_home("~/clashctl", Path::new("/home/alice")),
            PathBuf::from("/home/alice/clashctl")
        );
        assert_eq!(
            expand_home("~", Path::new("/home/alice")),
            PathBuf::from("/home/alice")
        );
    }

    #[test]
    fn resource_paths_follow_clashctl_install_resolution_order() {
        let candidates = install_env_candidates_from_sources(
            Some(install_env(
                "/home/alice/custom",
                Some("proc-service"),
                None,
            )),
            Some(install_env(
                "/opt/user-clash",
                Some("user-service"),
                Some("user-kernel"),
            )),
            Some(install_env(
                "/srv/system-clash",
                None,
                Some("system-kernel"),
            )),
            Some(Path::new("/home/alice")),
        );

        assert_eq!(
            base_dirs(&candidates),
            vec![
                PathBuf::from("/home/alice/custom"),
                PathBuf::from("/opt/user-clash"),
                PathBuf::from("/srv/system-clash"),
                PathBuf::from("/home/alice/clashctl"),
                PathBuf::from("/home/alice/.clashctl"),
            ]
        );
        assert_eq!(candidates[0].service_name.as_deref(), Some("proc-service"));
        assert_eq!(candidates[1].kernel_name.as_deref(), Some("user-kernel"));
    }

    #[test]
    fn resource_paths_deduplicate_overlapping_sources() {
        let candidates = install_env_candidates_from_sources(
            Some(InstallEnv::new(PathBuf::from("/home/alice/clashctl"))),
            Some(install_env(
                "/home/alice/clashctl",
                Some("user-service"),
                Some("user-kernel"),
            )),
            Some(install_env(
                "/home/alice/clashctl",
                Some("system-service"),
                Some("system-kernel"),
            )),
            Some(Path::new("/home/alice")),
        );

        assert_eq!(
            base_dirs(&candidates),
            vec![
                PathBuf::from("/home/alice/clashctl"),
                PathBuf::from("/home/alice/.clashctl"),
            ]
        );
        assert_eq!(candidates[0].service_name.as_deref(), Some("user-service"));
        assert_eq!(candidates[0].kernel_name.as_deref(), Some("user-kernel"));
    }

    #[test]
    fn resource_paths_use_system_marker_when_user_marker_is_absent() {
        let candidates = install_env_candidates_from_sources(
            None,
            None,
            Some(install_env(
                "/srv/system-clash",
                Some("system-service"),
                Some("system-kernel"),
            )),
            Some(Path::new("/home/alice")),
        );

        assert_eq!(
            base_dirs(&candidates),
            vec![
                PathBuf::from("/srv/system-clash"),
                PathBuf::from("/home/alice/clashctl"),
                PathBuf::from("/home/alice/.clashctl"),
            ]
        );
        assert_eq!(
            candidates[0].service_name.as_deref(),
            Some("system-service")
        );
        assert_eq!(candidates[0].kernel_name.as_deref(), Some("system-kernel"));
    }

    #[test]
    fn install_marker_parses_base_service_and_kernel() {
        let root = std::env::temp_dir().join(format!("clash-tui-marker-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create marker dir");
        let marker = root.join("install.env");
        fs::write(
            &marker,
            r#"
CLASH_BASE_DIR="~/custom-clash"
SERVICE_NAME='clashctl-main'
KERNEL_NAME=mihomo-custom
"#,
        )
        .expect("write install marker");

        let got =
            read_install_marker_file(&marker, Some(Path::new("/home/alice"))).expect("marker");
        assert_eq!(got.base_dir, PathBuf::from("/home/alice/custom-clash"));
        assert_eq!(got.service_name.as_deref(), Some("clashctl-main"));
        assert_eq!(got.kernel_name.as_deref(), Some("mihomo-custom"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn active_install_env_prefers_existing_install_state() {
        let root =
            std::env::temp_dir().join(format!("clash-tui-active-base-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let missing = root.join("missing");
        let installed = root.join("installed");
        fs::create_dir_all(&installed).expect("create installed dir");
        fs::write(installed.join("install-state.json"), "{}").expect("write install-state");

        let got = active_install_env_from_candidates(vec![
            install_env(missing, Some("missing-service"), None),
            install_env(installed.clone(), Some("clashctl-main"), Some("mihomo")),
        ])
        .expect("active install env");
        assert_eq!(got.base_dir, installed);
        assert_eq!(got.service_name.as_deref(), Some("clashctl-main"));
        assert_eq!(got.kernel_name.as_deref(), Some("mihomo"));

        let _ = fs::remove_dir_all(&root);
    }
}

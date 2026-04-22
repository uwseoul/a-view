use crate::types::{PortCategory, PortEntry, PortScanResult};
use chrono::Utc;

pub fn scan_ports() -> Result<PortScanResult, String> {
    let entries = listeners::get_all()
        .map_err(|e| format!("Failed to enumerate listening ports: {e}"))?;

    let ports: Vec<PortEntry> = entries
        .into_iter()
        .filter(|l| matches!(l.protocol, listeners::Protocol::TCP))
        .map(|l| {
            let process_name = if l.process.name.is_empty() {
                None
            } else {
                Some(l.process.name.clone())
            };
            let category = classify_process(process_name.as_deref().unwrap_or(""));
            PortEntry {
                protocol: l.protocol.to_string(),
                local_addr: l.socket.to_string(),
                port: l.socket.port(),
                pid: Some(l.process.pid),
                process_name,
                category,
            }
        })
        .collect();

    let total_count = ports.len() as i64;
    Ok(PortScanResult {
        ports,
        scanned_at: Utc::now().to_rfc3339(),
        total_count,
    })
}

pub fn classify_process(process_name: &str) -> PortCategory {
    let name = process_name.to_lowercase();
    let name = name.strip_suffix(".exe").unwrap_or(&name);

    match name {
        "nginx" | "apache" | "httpd" | "caddy" | "lighttpd" | "tomcat" => PortCategory::WebServer,
        "mysql" | "postgres" | "redis" | "mongo" | "mariadb" | "sqlite3"
        | "mongod" | "mysqld" | "redis-server" => PortCategory::Database,
        "node" | "python" | "java" | "dotnet" | "ruby" | "go" | "cargo" | "php"
        | "puma" | "uvicorn" | "gunicorn" => PortCategory::Development,
        "svchost" | "system" | "kernel_task" | "launchd" | "systemd" | "init"
        | "kthreadd" | "dwm" => PortCategory::System,
        _ => PortCategory::Other,
    }
}

fn extract_port(local_addr: &str) -> u16 {
    local_addr
        .rfind(':')
        .and_then(|i| local_addr[i + 1..].parse().ok())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PortCategory;

    #[test]
    fn test_classify_web_server() {
        assert_eq!(classify_process("nginx"), PortCategory::WebServer);
        assert_eq!(classify_process("caddy"), PortCategory::WebServer);
        assert_eq!(classify_process("apache"), PortCategory::WebServer);
        assert_eq!(classify_process("httpd"), PortCategory::WebServer);
        assert_eq!(classify_process("Nginx"), PortCategory::WebServer);
    }

    #[test]
    fn test_classify_database() {
        assert_eq!(classify_process("mysql"), PortCategory::Database);
        assert_eq!(classify_process("redis"), PortCategory::Database);
        assert_eq!(classify_process("postgres"), PortCategory::Database);
        assert_eq!(classify_process("mongod"), PortCategory::Database);
        assert_eq!(classify_process("mysqld"), PortCategory::Database);
    }

    #[test]
    fn test_classify_development() {
        assert_eq!(classify_process("node"), PortCategory::Development);
        assert_eq!(classify_process("python"), PortCategory::Development);
        assert_eq!(classify_process("cargo"), PortCategory::Development);
        assert_eq!(classify_process("uvicorn"), PortCategory::Development);
    }

    #[test]
    fn test_classify_system() {
        assert_eq!(classify_process("svchost"), PortCategory::System);
        assert_eq!(classify_process("launchd"), PortCategory::System);
        assert_eq!(classify_process("systemd"), PortCategory::System);
        assert_eq!(classify_process("dwm"), PortCategory::System);
    }

    #[test]
    fn test_classify_other() {
        assert_eq!(classify_process("unknown"), PortCategory::Other);
        assert_eq!(classify_process("chrome"), PortCategory::Other);
        assert_eq!(classify_process(""), PortCategory::Other);
    }

    #[test]
    fn test_extract_port() {
        assert_eq!(extract_port("127.0.0.1:3000"), 3000);
        assert_eq!(extract_port("[::]:80"), 80);
        assert_eq!(extract_port("0.0.0.0:443"), 443);
        assert_eq!(extract_port("192.168.1.1:8080"), 8080);
    }
}

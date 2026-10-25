//! A small blocking Docker Engine API client (decision 0008): HTTP/1.1 over
//! a Unix socket, TCP, or a Windows named pipe. Requests use API v1.41 paths
//! so response fields keep that version's meaning on newer engines.

use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use kipple_core::platform::Env;

pub const API: &str = "/v1.41";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Endpoint {
    Unix(PathBuf),
    Tcp(String),
    Pipe(String),
}

#[derive(Debug, thiserror::Error)]
pub enum DockerError {
    #[error("Docker isn't running or can't be reached ({0})")]
    Unreachable(String),
    #[error("Docker answered {status}: {message}")]
    Status { status: u16, message: String },
    #[error("unexpected answer from Docker: {0}")]
    Protocol(String),
    #[error("{0} isn't supported yet; set DOCKER_HOST to a local socket")]
    Unsupported(String),
}

impl Endpoint {
    /// Parses `unix:///path`, `tcp://host:port`, `npipe:////./pipe/name`.
    pub fn parse(host: &str) -> Result<Self, DockerError> {
        if let Some(p) = host.strip_prefix("unix://") {
            Ok(Endpoint::Unix(PathBuf::from(p)))
        } else if let Some(a) = host.strip_prefix("tcp://") {
            Ok(Endpoint::Tcp(a.trim_end_matches('/').to_owned()))
        } else if let Some(p) = host.strip_prefix("npipe://") {
            Ok(Endpoint::Pipe(p.replace('/', "\\")))
        } else {
            Err(DockerError::Unsupported(format!("DOCKER_HOST `{host}`")))
        }
    }
}

/// Where to find the daemon (INTEGRATIONS.md §4.5): `DOCKER_HOST`, then the
/// active context, then the platform defaults that exist.
pub fn endpoints(env: &Env) -> Vec<Endpoint> {
    if let Some(h) = env.get_str("DOCKER_HOST") {
        return Endpoint::parse(h).into_iter().collect();
    }
    let mut out = Vec::new();
    let docker_dir = env
        .get_path("DOCKER_CONFIG")
        .or_else(|| env.home().map(|h| h.join(".docker")));
    if let Some(dir) = &docker_dir
        && let Some(host) = context_host(dir)
        && let Ok(e) = Endpoint::parse(&host)
    {
        out.push(e);
    }
    if cfg!(windows) {
        out.push(Endpoint::Pipe(r"\\.\pipe\docker_engine".into()));
    } else {
        if let Some(dir) = &docker_dir {
            out.push(Endpoint::Unix(dir.join("run/docker.sock")));
        }
        out.push(Endpoint::Unix(PathBuf::from("/var/run/docker.sock")));
        if let Some(rt) = env.get_path("XDG_RUNTIME_DIR") {
            out.push(Endpoint::Unix(rt.join("docker.sock")));
        }
    }
    out.dedup();
    out
}

/// Host of the current context from `config.json` + `contexts/meta`.
fn context_host(docker_dir: &Path) -> Option<String> {
    let config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(docker_dir.join("config.json")).ok()?)
            .ok()?;
    let name = config.get("currentContext")?.as_str()?;
    if name == "default" {
        return None;
    }
    for meta in crate::util::children(&docker_dir.join("contexts/meta")) {
        let Ok(text) = std::fs::read_to_string(meta.join("meta.json")) else {
            continue;
        };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else {
            continue;
        };
        if v.get("Name").and_then(|n| n.as_str()) == Some(name) {
            return v
                .pointer("/Endpoints/docker/Host")
                .and_then(|h| h.as_str())
                .map(str::to_owned);
        }
    }
    None
}

trait Stream: Read + Write {}
impl<T: Read + Write> Stream for T {}

pub struct Client {
    endpoint: Endpoint,
    timeout: Duration,
}

pub struct Response {
    pub status: u16,
    pub body: Vec<u8>,
}

impl Client {
    pub fn new(endpoint: Endpoint) -> Self {
        Self {
            endpoint,
            timeout: Duration::from_secs(30),
        }
    }

    /// The first endpoint that answers `/_ping`.
    pub fn connect(env: &Env) -> Result<Self, DockerError> {
        let mut last = DockerError::Unreachable("no Docker endpoint found".into());
        for e in endpoints(env) {
            let c = Client::new(e);
            match c.request("GET", "/_ping") {
                Ok(r) if r.status == 200 => return Ok(c),
                Ok(r) => {
                    last = DockerError::Status {
                        status: r.status,
                        message: "ping failed".into(),
                    }
                }
                Err(e) => last = e,
            }
        }
        Err(last)
    }

    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    fn open(&self) -> Result<Box<dyn Stream>, DockerError> {
        let unreachable = |e: std::io::Error| DockerError::Unreachable(e.to_string());
        match &self.endpoint {
            #[cfg(unix)]
            Endpoint::Unix(p) => {
                let s = std::os::unix::net::UnixStream::connect(p).map_err(unreachable)?;
                s.set_read_timeout(Some(self.timeout))
                    .map_err(unreachable)?;
                Ok(Box::new(s))
            }
            #[cfg(not(unix))]
            Endpoint::Unix(p) => Err(DockerError::Unsupported(format!(
                "unix socket {}",
                p.display()
            ))),
            Endpoint::Tcp(addr) => {
                let s = std::net::TcpStream::connect(addr).map_err(unreachable)?;
                s.set_read_timeout(Some(self.timeout))
                    .map_err(unreachable)?;
                Ok(Box::new(s))
            }
            Endpoint::Pipe(name) => {
                let f = std::fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(name)
                    .map_err(unreachable)?;
                Ok(Box::new(f))
            }
        }
    }

    /// Sends one request (`path` without the API prefix) and reads the
    /// whole response.
    pub fn request(&self, method: &str, path: &str) -> Result<Response, DockerError> {
        let mut s = self.open()?;
        let full = if path == "/_ping" {
            path.to_owned()
        } else {
            format!("{API}{path}")
        };
        let req = format!(
            "{method} {full} HTTP/1.1\r\nHost: docker\r\nUser-Agent: kipple/{}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            kipple_core::VERSION
        );
        s.write_all(req.as_bytes())
            .map_err(|e| DockerError::Unreachable(e.to_string()))?;
        read_response(BufReader::new(s))
    }

    /// GET returning JSON.
    pub fn get_json(&self, path: &str) -> Result<serde_json::Value, DockerError> {
        let r = self.request("GET", path)?;
        check(&r)?;
        serde_json::from_slice(&r.body).map_err(|e| DockerError::Protocol(e.to_string()))
    }

    /// DELETE/POST that must succeed.
    pub fn send(&self, method: &str, path: &str) -> Result<serde_json::Value, DockerError> {
        let r = self.request(method, path)?;
        check(&r)?;
        if r.body.is_empty() {
            Ok(serde_json::Value::Null)
        } else {
            serde_json::from_slice(&r.body).map_err(|e| DockerError::Protocol(e.to_string()))
        }
    }
}

fn check(r: &Response) -> Result<(), DockerError> {
    if (200..300).contains(&r.status) {
        return Ok(());
    }
    let message = serde_json::from_slice::<serde_json::Value>(&r.body)
        .ok()
        .and_then(|v| v.get("message").and_then(|m| m.as_str()).map(str::to_owned))
        .unwrap_or_else(|| String::from_utf8_lossy(&r.body).into_owned());
    Err(DockerError::Status {
        status: r.status,
        message,
    })
}

fn read_response(mut r: impl BufRead) -> Result<Response, DockerError> {
    let proto = |m: &str| DockerError::Protocol(m.to_owned());
    let mut line = String::new();
    r.read_line(&mut line)
        .map_err(|e| DockerError::Unreachable(e.to_string()))?;
    let status: u16 = line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| proto("bad status line"))?;
    let mut chunked = false;
    let mut length: Option<usize> = None;
    loop {
        line.clear();
        r.read_line(&mut line)
            .map_err(|e| DockerError::Protocol(e.to_string()))?;
        let l = line.trim_end();
        if l.is_empty() {
            break;
        }
        if let Some((k, v)) = l.split_once(':') {
            let (k, v) = (k.trim().to_ascii_lowercase(), v.trim());
            if k == "transfer-encoding" && v.eq_ignore_ascii_case("chunked") {
                chunked = true;
            } else if k == "content-length" {
                length = v.parse().ok();
            }
        }
    }
    let mut body = Vec::new();
    if chunked {
        loop {
            line.clear();
            r.read_line(&mut line)
                .map_err(|e| DockerError::Protocol(e.to_string()))?;
            let size = usize::from_str_radix(line.trim().split(';').next().unwrap_or("0"), 16)
                .map_err(|_| proto("bad chunk size"))?;
            if size == 0 {
                break;
            }
            let mut chunk = vec![0; size];
            r.read_exact(&mut chunk)
                .map_err(|e| DockerError::Protocol(e.to_string()))?;
            body.extend(chunk);
            line.clear();
            r.read_line(&mut line)
                .map_err(|e| DockerError::Protocol(e.to_string()))?;
        }
    } else if let Some(n) = length {
        body.resize(n, 0);
        r.read_exact(&mut body)
            .map_err(|e| DockerError::Protocol(e.to_string()))?;
    } else {
        r.read_to_end(&mut body)
            .map_err(|e| DockerError::Protocol(e.to_string()))?;
    }
    Ok(Response { status, body })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_endpoints() {
        assert_eq!(
            Endpoint::parse("unix:///var/run/docker.sock").unwrap(),
            Endpoint::Unix("/var/run/docker.sock".into())
        );
        assert_eq!(
            Endpoint::parse("tcp://127.0.0.1:2375").unwrap(),
            Endpoint::Tcp("127.0.0.1:2375".into())
        );
        assert!(Endpoint::parse("ssh://host").is_err());
    }

    #[test]
    fn reads_chunked_and_sized_bodies() {
        let raw = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n4\r\n{\"a\"\r\n3\r\n:1}\r\n0\r\n\r\n";
        let r = read_response(&raw[..]).unwrap();
        assert_eq!(r.body, b"{\"a\":1}");
        let raw = b"HTTP/1.1 404 Not Found\r\nContent-Length: 15\r\n\r\n{\"message\":\"x\"}";
        let r = read_response(&raw[..]).unwrap();
        assert_eq!(r.status, 404);
        assert!(matches!(
            check(&r),
            Err(DockerError::Status { status: 404, .. })
        ));
    }

    #[test]
    fn docker_host_wins() {
        let env = Env::from_pairs([("HOME", "/h"), ("DOCKER_HOST", "tcp://x:1")]);
        assert_eq!(endpoints(&env), vec![Endpoint::Tcp("x:1".into())]);
    }

    #[test]
    fn reads_current_context() {
        let home = kipple_core::testing::TestHome::new().unwrap();
        home.write(".docker/config.json", r#"{"currentContext":"colima"}"#)
            .unwrap();
        home.write(
            ".docker/contexts/meta/abc/meta.json",
            r#"{"Name":"colima","Endpoints":{"docker":{"Host":"unix:///h/.colima/docker.sock"}}}"#,
        )
        .unwrap();
        let mut env = home.env().clone();
        env.remove("DOCKER_HOST");
        assert_eq!(
            endpoints(&env)[0],
            Endpoint::Unix("/h/.colima/docker.sock".into())
        );
    }
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransportType {
    CLIServer(CLIServer),
    ServerSentEvents(ServerSentEvents),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CLIServer {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub cwd_parameter: Option<String>,
    /// Static env vars added via editor inputs.
    pub static_env_vars: Vec<StaticEnvVar>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaticEnvVar {
    pub name: String,
    /// To avoid leaking environment variables, we ensure that values are not
    /// serialized before being sent to our servers
    #[serde(skip_serializing, default)]
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaticHeader {
    pub name: String,
    /// To avoid leaking header values (which may contain secrets), we ensure that values are not
    /// serialized before being sent to our servers
    #[serde(skip_serializing, default)]
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerSentEvents {
    pub url: String,
    /// Static headers added via editor inputs.
    #[serde(default)]
    pub headers: Vec<StaticHeader>,
}

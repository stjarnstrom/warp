use std::sync::Arc;

type RemoteServerIdentityKeyFn = dyn Fn() -> String + Send + Sync;

/// Installation identity and privacy preferences for SSH daemons.
/// The identity partitions sockets and process directories; SSH provides authentication.
#[derive(Clone)]
pub struct RemoteServerAuthContext {
    remote_server_identity_key: Arc<RemoteServerIdentityKeyFn>,
    crash_reporting_enabled: bool,
}

impl RemoteServerAuthContext {
    pub fn new(
        remote_server_identity_key: impl Fn() -> String + Send + Sync + 'static,
        crash_reporting_enabled: bool,
    ) -> Self {
        Self {
            remote_server_identity_key: Arc::new(remote_server_identity_key),
            crash_reporting_enabled,
        }
    }

    pub fn remote_server_identity_key(&self) -> String {
        (self.remote_server_identity_key)()
    }

    pub fn crash_reporting_enabled(&self) -> bool {
        self.crash_reporting_enabled
    }
}

//! Redis enforcing the Deploiment chart's exact ACL (MAIR-267). Local copy of
//! `mairie360_api_lib::test_setup::redis_setup::start_acl_redis_container` (unreleased lib):
//! switch to it when the lib is bumped.

use testcontainers::core::{ContainerPort, WaitFor};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, GenericImage, ImageExt};

pub const ACL_PASSWORD: &str = "acl-test-password";

/// Same lines as the chart's `entrypoint.sh`: `default` off, `admin`, `core-api` (read-write on
/// `revoked:*`), `project-api` (read-only on `revoked:*`).
fn chart_acl_lines() -> Vec<String> {
    let commands = "+get +set +del +exists +expire";
    vec![
        "default off resetkeys resetchannels -@all".to_string(),
        format!("admin on >{ACL_PASSWORD} ~* &* allcommands"),
        format!("core-api on >{ACL_PASSWORD} ~core-api:* ~revoked:* resetchannels -@all {commands}"),
        format!(
            "project-api on >{ACL_PASSWORD} ~project-api:* %R~revoked:* resetchannels -@all {commands}"
        ),
    ]
}

pub struct AclRedis {
    _node: ContainerAsync<GenericImage>,
    host: String,
    port: u16,
}

impl AclRedis {
    pub async fn start() -> Self {
        let mut cmd = vec!["redis-server".to_string()];
        for line in chart_acl_lines() {
            cmd.push("--user".to_string());
            cmd.extend(line.split(' ').map(str::to_string));
        }
        let node = GenericImage::new("redis", "7.4-alpine")
            .with_exposed_port(ContainerPort::Tcp(6379))
            .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
            .with_cmd(cmd)
            .start()
            .await
            .expect("Failed to start Redis with ACL");
        let host = node.get_host().await.unwrap().to_string();
        let port = node.get_host_port_ipv4(6379).await.unwrap();
        Self {
            _node: node,
            host,
            port,
        }
    }

    /// URL authenticated as `user`.
    pub fn url_as(&self, user: &str) -> String {
        format!("redis://{user}:{ACL_PASSWORD}@{}:{}", self.host, self.port)
    }

    /// Raw connection as `admin`, to inspect the keys really written.
    pub fn admin(&self) -> redis::Connection {
        redis::Client::open(self.url_as("admin"))
            .unwrap()
            .get_connection()
            .unwrap()
    }
}

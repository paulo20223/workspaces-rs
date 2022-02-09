use super::{
    Account, AllowDevAccountCreation, AllowStatePatching, CallExecution, Contract, NetworkClient,
    NetworkInfo, TopLevelAccountCreator,
};
use crate::network::sandbox::{HasRpcPort, DEFAULT_DEPOSIT};
use crate::network::server::SandboxServer;
use crate::network::{Info, Sandbox};
use crate::rpc::client::Client;
use crate::types::{AccountId, InMemorySigner, SecretKey};
use async_mutex::Mutex;
use async_trait::async_trait;
use once_cell::sync::Lazy;
use std::path::PathBuf;
use std::str::FromStr;

static SHARED_SERVER: Lazy<SandboxServer> = Lazy::new(|| {
    let mut server = SandboxServer::default();
    server.start().unwrap();
    server
});

// Using a shared sandbox instance is thread-safe as long as all threads use it with their own
// account. This means, however, is that the creation of these accounts should be sequential in
// order to avoid duplicated nonces.
static TLA_ACCOUNT_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

pub struct SandboxShared {
    client: Client,
    info: Info,
}

impl SandboxShared {
    pub(crate) fn new() -> Self {
        let client = Client::new(SHARED_SERVER.rpc_addr());
        let info = Info {
            name: "sandbox-shared".to_string(),
            root_id: AccountId::from_str("test.near").unwrap(),
            keystore_path: PathBuf::from(".near-credentials/sandbox/"),
            rpc_url: SHARED_SERVER.rpc_addr(),
        };

        Self { client, info }
    }
}

impl AllowStatePatching for SandboxShared {}

impl AllowDevAccountCreation for SandboxShared {}

#[async_trait]
impl TopLevelAccountCreator for SandboxShared {
    async fn create_tla(
        &self,
        id: AccountId,
        sk: SecretKey,
    ) -> anyhow::Result<CallExecution<Account>> {
        let root_signer = Sandbox::root_signer(self.rpc_port());
        let guard = TLA_ACCOUNT_MUTEX.lock().await;
        let outcome = self
            .client
            .create_account(&root_signer, &id, sk.public_key(), DEFAULT_DEPOSIT)
            .await?;
        drop(guard);

        let signer = InMemorySigner::from_secret_key(id.clone(), sk);
        Ok(CallExecution {
            result: Account::new(id, signer),
            details: outcome.into(),
        })
    }

    async fn create_tla_and_deploy(
        &self,
        id: AccountId,
        sk: SecretKey,
        wasm: &[u8],
    ) -> anyhow::Result<CallExecution<Contract>> {
        let root_signer = Sandbox::root_signer(self.rpc_port());
        let guard = TLA_ACCOUNT_MUTEX.lock().await;
        let outcome = self
            .client
            .create_account_and_deploy(
                &root_signer,
                &id,
                sk.public_key(),
                DEFAULT_DEPOSIT,
                wasm.into(),
            )
            .await?;
        drop(guard);

        let signer = InMemorySigner::from_secret_key(id.clone(), sk);
        Ok(CallExecution {
            result: Contract::new(id, signer),
            details: outcome.into(),
        })
    }
}

impl NetworkClient for SandboxShared {
    fn client(&self) -> &Client {
        &self.client
    }
}

impl NetworkInfo for SandboxShared {
    fn info(&self) -> &Info {
        &self.info
    }
}

impl HasRpcPort for SandboxShared {
    fn rpc_port(&self) -> u16 {
        SHARED_SERVER.rpc_port
    }
}
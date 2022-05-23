#![deny(missing_docs)]
#![doc = include_str!("../README.md")]
use std::path::Path;

use ic_agent::ic_types::Principal;
use ic_agent::identity::BasicIdentity;
use ic_agent::{agent::http_transport::ReqwestHttpReplicaV2Transport, identity::PemError};
use candid::utils::ArgumentEncoder;

pub use ic_agent::Agent;

mod errors;
pub use errors::{Error, Result};

pub mod canister;

pub use canister::{Canister, Management, ManagementCanister, Wallet, WalletCanister};

const URL: &str = "http://localhost:8000";

/// Get the identity for an account.
/// This is useful for testing.
///
/// If this is ever needed outside of `get_agent` just make this
/// function public.
pub fn get_identity(account_name: impl AsRef<Path>) -> Result<BasicIdentity> {
    let mut ident_path = dirs::home_dir().ok_or(crate::Error::MissingConfig)?;
    ident_path.push(".config");
    ident_path.push("dfx/identity");
    ident_path.push(account_name);
    ident_path.push("identity.pem");
    match BasicIdentity::from_pem_file(&ident_path) {
        Ok(identity) => Ok(identity),
        Err(PemError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {
            Err(Error::CertNotFound(ident_path))
        }
        Err(err) => Err(Error::from(err)),
    }
}

/// Get an agent by identity name.
///
/// This is assuming there is an agent identity available.
/// If no identities area available then clone the correct **identity** project.
///
/// ```text
/// # Clone the identity project first
/// mkdir -p ~/.config/dfx/identity/
/// cp -Rn ./identity/.config/dfx/identity/* ~/.config/dfx/identity/
/// ```
pub async fn get_agent(name: impl Into<&str>, url: Option<&str>) -> Result<Agent> {
    let identity = get_identity(name.into())?;

    let url = url.unwrap_or(URL);
    let transport = ReqwestHttpReplicaV2Transport::create(url)?;

    let agent = Agent::builder()
        .with_transport(transport)
        .with_identity(identity)
        .build()?;

    agent.fetch_root_key().await?;

    Ok(agent)
}

/// Create a default `Delay` with a throttle of 500ms
/// and a timout of five minutes.
pub fn get_waiter() -> garcon::Delay {
    garcon::Delay::builder()
        .throttle(std::time::Duration::from_millis(500))
        .timeout(std::time::Duration::from_secs(60 * 5))
        .build()
}

/// Create a canister and install
/// the provided byte code.
pub async fn create_canister<T: ArgumentEncoder>(
    agent: &Agent,
    account_name: impl AsRef<str>,
    bytecode: Vec<u8>,
    arg: T,
    cycles: u64,
) -> Result<Principal> {
    let wallet = Canister::new_wallet(agent, account_name, None)?;
    let management = Canister::new_management(agent);
    let canister_id = wallet.create_canister(cycles, None).await?;
    management
        .install_code(&wallet, canister_id, bytecode, arg)
        .await?;
    Ok(canister_id)
}

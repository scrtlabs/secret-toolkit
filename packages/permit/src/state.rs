use cosmwasm_std::{HumanAddr, Storage};

pub struct RevokedPermits;

impl RevokedPermits {
    pub fn is_permit_revoked(
        storage: &dyn Storage,
        storage_prefix: &str,
        account: &HumanAddr,
        permit_name: &str,
    ) -> bool {
        let storage_key = storage_prefix.to_string() + &account.to_string() + permit_name;

        storage.get(storage_key.as_bytes()).is_some()
    }

    pub fn revoke_permit(
        storage: &mut dyn Storage,
        storage_prefix: &str,
        account: &HumanAddr,
        permit_name: &str,
    ) {
        let storage_key = storage_prefix.to_string() + &account.to_string() + permit_name;

        storage.set(storage_key.as_bytes(), &[])
    }
}

use cosmwasm_std::{HumanAddr, Storage};

pub const PREFIX_REVOKED_PERMITS: &str = "revoked_permits";

pub struct RevokedPemits;

impl RevokedPemits {
    pub fn is_permit_revoked(
        storgae: &dyn Storage,
        account: &HumanAddr,
        permit_name: &str,
    ) -> bool {
        let storage_key = PREFIX_REVOKED_PERMITS.to_string() + &account.to_string() + permit_name;

        storgae.get(storage_key.as_bytes()).is_some()
    }

    pub fn revoke_permit(storage: &mut dyn Storage, account: &HumanAddr, permit_name: &str) {
        let storage_key = PREFIX_REVOKED_PERMITS.to_string() + &account.to_string() + permit_name;

        storage.set(storage_key.as_bytes(), &[])
    }
}

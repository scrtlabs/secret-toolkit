use cosmwasm_std::Storage;

pub struct RevokedPermits;

impl RevokedPermits {
    pub fn is_permit_revoked(
        storgae: &dyn Storage,
        storage_prefix: &str,
        account: &str,
        permit_name: &str,
    ) -> bool {
        let storage_key = storage_prefix.to_string() + account + permit_name;

        storgae.get(storage_key.as_bytes()).is_some()
    }

    pub fn revoke_permit(
        storage: &mut dyn Storage,
        storage_prefix: &str,
        account: &str,
        permit_name: &str,
    ) {
        let storage_key = storage_prefix.to_string() + account + permit_name;

        storage.set(storage_key.as_bytes(), &[])
    }
}

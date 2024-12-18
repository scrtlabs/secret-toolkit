use cosmwasm_std::{StdError, StdResult, Storage, Uint64};
use schemars::JsonSchema;
use secret_toolkit_storage::{Item, Keymap};
use serde::{Deserialize, Serialize};

use crate::REVOKED_ALL;

/// This is the default implementation of the revoked permits store, using the "revoked_permits"
/// storage prefix for named permits and "all_revoked_permits" for revoked blanket permits.
/// It also sets the maximum number of all permit revocations to 10 by default.
///
/// You can use different storage locations and parameters by implementing `RevokedPermitsStore` 
/// for your own type.
pub struct RevokedPermits;

impl<'a> RevokedPermitsStore<'a> for RevokedPermits {
    const NAMED_REVOKED_PERMITS_PREFIX: &'static [u8] = b"revoked_permits";
    const ALL_REVOKED_PERMITS: Keymap<'a, u64, StoredAllRevokedInterval> = Keymap::new(b"__all_revoked_permits__1");
    const ALL_REVOKED_NEXT_ID: Item<'a, u64> = Item::new(b"__all_revoked_permits_serial_id__1");
    const MAX_ALL_REVOKED_INTERVALS: Option<u8> = Some(10);
    const ALL_TIME_REVOKED_ALL:Item<'a, bool> = Item::new(b"__all_time_revoked_all__1");
}

/// A trait describing the interface of a RevokedPermits store/vault.
///
/// It includes a default implementation that only requires specifying where in the storage
/// the keys should be held.
pub trait RevokedPermitsStore<'a> {
    const NAMED_REVOKED_PERMITS_PREFIX: &'static [u8];
    const ALL_REVOKED_PERMITS: Keymap<'a, u64, StoredAllRevokedInterval>;
    const ALL_REVOKED_NEXT_ID: Item<'a, u64>;
    const MAX_ALL_REVOKED_INTERVALS: Option<u8>;
    const ALL_TIME_REVOKED_ALL:Item<'a, bool>;

    /// returns a bool indicating if a named permit is revoked
    fn is_permit_revoked(
        storage: &dyn Storage,
        account: &str,
        permit_name: &str,
    ) -> bool {
        let mut storage_key = Vec::new();
        storage_key.extend_from_slice(Self::NAMED_REVOKED_PERMITS_PREFIX);
        storage_key.extend_from_slice(account.as_bytes());
        storage_key.extend_from_slice(permit_name.as_bytes());

        storage.get(&storage_key).is_some()
    }

    /// revokes a named permit permanently
    fn revoke_permit(
        storage: &mut dyn Storage,
        account: &str,
        permit_name: &str,
    ) {
        let mut storage_key = Vec::new();
        storage_key.extend_from_slice(Self::NAMED_REVOKED_PERMITS_PREFIX);
        storage_key.extend_from_slice(account.as_bytes());
        storage_key.extend_from_slice(permit_name.as_bytes());

        // Since cosmwasm V1.0 it's not possible to set an empty value, hence set some unimportant
        // character '_'
        //
        // Here is the line of the new panic that was added when trying to insert an empty value:
        // https://github.com/scrtlabs/cosmwasm/blob/f7e2b1dbf11e113e258d796288752503a5012367/packages/std/src/storage.rs#L30
        storage.set(&storage_key, "_".as_bytes())
    }

    /// revokes all permits created after and before
    fn revoke_all_permits(
        storage: &mut dyn Storage,
        account: &str,
        interval: &AllRevokedInterval,
    ) -> StdResult<String> {
        // Check if *neither* `created_before` or `created_after` is supplied
        // In this case, we are globally turning off all permits for this address, which
        // makes it so ANY permit will be rejected. This special case does not count
        // toward the maximum number of revoked intervals.
        if interval.created_before.is_none() && interval.created_after.is_none() {
            // get all time revocations store for this account
            let all_time_revoked_store = Self::ALL_TIME_REVOKED_ALL.add_suffix(account.as_bytes());
            
            // set all time revocations to true, this is idempotent
            all_time_revoked_store.save(storage, &true)?;

            // return a revocation ID of "REVOKED_ALL"
            return Ok(REVOKED_ALL.to_string());
        }


        // get the revocations store for this account
        let all_revocations_store = Self::ALL_REVOKED_PERMITS.add_suffix(account.as_bytes());

        // check that maximum number of revocations has not been met
        if let Some(max_revocations) = Self::MAX_ALL_REVOKED_INTERVALS {
            if all_revocations_store.get_len(storage)? >= max_revocations.into() {
                return Err(StdError::generic_err(
                    format!("Maximum number of permit revocations ({}) has been met", max_revocations)
                ));
            }
        }

        // get the next id store for this account
        let next_id_store = Self::ALL_REVOKED_NEXT_ID.add_suffix(account.as_bytes());

        // get the next id
        let next_id = next_id_store.may_load(storage)?.unwrap_or_default();

        // store the revocation
        all_revocations_store.insert(storage, &next_id, &interval.into_stored())?;

        // increment next id
        next_id_store.save(storage, &(next_id.wrapping_add(1)))?;

        Ok(format!("{}", next_id))
    }

    /// deletes the permit revocation with the given id for this account
    fn delete_revocation(
        storage: &mut dyn Storage,
        account: &str,
        id: &str,
    ) -> StdResult<()> {
        // check if this is the all time special case
        if id == REVOKED_ALL {
            // get all time revocations store for this account
            let all_time_revoked_store = Self::ALL_TIME_REVOKED_ALL.add_suffix(account.as_bytes());
            
            // set all time revocations to false
            all_time_revoked_store.save(storage, &false)?;

            // return
            return Ok(())
        }

        // get the revocations store for this account
        let all_revocations_store = Self::ALL_REVOKED_PERMITS.add_suffix(account.as_bytes());

        // try to convert id to a u64
        let Ok(id_str) = u64::from_str_radix(id, 10) else {
            return Err(StdError::generic_err("Deleted revocation id not Uint64"));
        };

        // remove the permit revocation with the given id
        all_revocations_store.remove(storage, &id_str)
    }

    /// lists all the revocations for the account
    /// returns a vec of revocations
    fn list_revocations(
        storage: &dyn Storage,
        account: &str,
    ) -> StdResult<Vec<AllRevocation>> {
        // get the revocations store for this account
        let all_revocations_store = Self::ALL_REVOKED_PERMITS.add_suffix(account.as_bytes());

        // select elements and convert to AllRevocation structs
        let mut result: Vec<AllRevocation> = all_revocations_store
            .iter(storage)?
            .filter_map(|r| {
                match r {
                    Ok(r) => Some(AllRevocation {
                        revocation_id: format!("{}", r.0),
                        interval: r.1.to_humanized()
                    }),
                    Err(_) => None
                }
            })
            .collect();

        // check if there is an all time revocation 
        if Self::is_all_time_revoked(storage, account)? {
            // add that to the result
            result.push(AllRevocation {
                revocation_id: REVOKED_ALL.to_string(),
                interval: AllRevokedInterval { 
                    created_before: None, 
                    created_after: None 
                }
            });
        }

        Ok(result)
    }

    /// returns bool if queries are all time revoked for this account
    fn is_all_time_revoked(storage: &dyn Storage, account: &str) -> StdResult<bool> {
        // get the all time revoked store for this account
        let all_time_revoked_store = Self::ALL_TIME_REVOKED_ALL.add_suffix(account.as_bytes());

        let result = all_time_revoked_store.may_load(storage)?.unwrap_or_default();

        Ok(result)
    }

}

/// An interval over which all permits will be rejected
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct AllRevokedInterval {
    pub created_before: Option<Uint64>,
    pub created_after: Option<Uint64>,
}

impl AllRevokedInterval {
    fn into_stored(&self) -> StoredAllRevokedInterval {
        StoredAllRevokedInterval { 
            created_before: self.created_before.and_then(|cb| Some(cb.u64())), 
            created_after: self.created_after.and_then(|ca| Some(ca.u64())), 
        }
    }
}

/// An interval over which all permits will be rejected
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct StoredAllRevokedInterval {
    pub created_before: Option<u64>,
    pub created_after: Option<u64>,
}

impl StoredAllRevokedInterval {
    fn to_humanized(&self) -> AllRevokedInterval {
        AllRevokedInterval {
            created_before: self.created_before.and_then(|cb| Some(Uint64::from(cb))), 
            created_after: self.created_after.and_then(|ca| Some(Uint64::from(ca))),
        }
    }
}

/// Revocation id and interval data struct
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct AllRevocation {
    pub revocation_id: String,
    pub interval: AllRevokedInterval,
}
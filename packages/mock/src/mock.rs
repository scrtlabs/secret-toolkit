use cosmwasm_std::{
    testing::{MockQuerier, MockStorage},
    Api, Binary, BlockInfo, CanonicalAddr, Coin, ContractInfo, Env, Extern, HumanAddr, MessageInfo,
    StdError, StdResult,
};

use bech32::{FromBase32, ToBase32};

pub const MOCK_CONTRACT_ADDR: &[u8] = &[
    59, 26, 116, 133, 198, 22, 44, 88, 131, 238, 69, 251, 45, 116, 119, 168, 125, 138, 76, 229,
];

/// All external requirements that can be injected for unit tests.
/// It sets the given balance for the contract itself, nothing else
///
/// This is an alternative to the function of the same name in `cosmwasm_std`,
/// that supports an `Api` that is very similar to the one found in real networks.
pub fn mock_dependencies(
    hrp: &'static str,
    contract_balance: &[Coin],
) -> Extern<MockStorage, MockApi, MockQuerier> {
    let api = MockApi::new(hrp);
    let contract_addr = api
        .human_address(&CanonicalAddr::from(MOCK_CONTRACT_ADDR))
        .unwrap();
    Extern {
        storage: MockStorage::default(),
        querier: MockQuerier::new(&[(&contract_addr, contract_balance)]),
        api,
    }
}

/// Initializes the querier along with the mock_dependencies.
/// Sets all balances provided (yoy must explicitly set contract balance if desired)
///
/// This is an alternative to the function of the same name in `cosmwasm_std`,
/// that supports an `Api` that is very similar to the one found in real networks.
pub fn mock_dependencies_with_balances(
    hrp: &'static str,
    balances: &[(&HumanAddr, &[Coin])],
) -> Extern<MockStorage, MockApi, MockQuerier> {
    Extern {
        storage: MockStorage::default(),
        querier: MockQuerier::new(balances),
        api: MockApi::new(hrp),
    }
}

/// Just set sender and sent funds for the message. The rest uses defaults.
/// The sender will be canonicalized internally to allow developers pasing in human readable senders.
/// This is intended for use in test code only.
///
/// This is an alternative to the function of the same name in `cosmwasm_std`,
/// that generates the contract address based on the hrp you choose.
pub fn mock_env<U: Into<HumanAddr>>(hrp: &'static str, sender: U, sent: &[Coin]) -> Env {
    let api = MockApi::new(hrp);
    let contract_addr = api
        .human_address(&CanonicalAddr::from(MOCK_CONTRACT_ADDR))
        .unwrap();
    Env {
        block: BlockInfo {
            height: 12_345,
            time: 1_571_797_419,
            chain_id: "cosmos-testnet-14002".to_string(),
        },
        message: MessageInfo {
            sender: sender.into(),
            sent_funds: sent.to_vec(),
        },
        contract: ContractInfo {
            address: contract_addr,
        },
        contract_key: Some("".to_string()),
        contract_code_hash: "".to_string(),
    }
}

/// This type is an alternative to the MockApi of `cosmwasm_std`.
/// It supports the same bech32 address format as Cosmos chains,
/// with a customizable HRP
#[derive(Copy, Clone)]
pub struct MockApi {
    hrp: &'static str,
}

impl MockApi {
    pub fn new(hrp: &'static str) -> Self {
        Self { hrp }
    }
}

impl Default for MockApi {
    fn default() -> Self {
        Self::new("secret")
    }
}

impl Api for MockApi {
    fn canonical_address(&self, human: &HumanAddr) -> StdResult<CanonicalAddr> {
        match bech32::decode(&human.0) {
            Ok((_hrp, data, _variant)) => {
                let data = Vec::from_base32(&data).map_err(|err| {
                    StdError::generic_err(format!(
                        "Could not decode address {:?}: {:?}",
                        human, err
                    ))
                })?;
                Ok(CanonicalAddr(Binary(data)))
            }
            Err(err) => Err(StdError::generic_err(format!(
                "Could not canonicalize address {:?}: {:?}",
                human, err
            ))),
        }
    }

    fn human_address(&self, canonical: &CanonicalAddr) -> StdResult<HumanAddr> {
        use bech32::Variant::Bech32;
        match bech32::encode(&self.hrp, &canonical.as_slice().to_base32(), Bech32) {
            Ok(addr) => Ok(HumanAddr(addr)),
            Err(err) => Err(StdError::generic_err(format!(
                "Could not humanize address {:?}: {:?}",
                canonical, err,
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api() -> StdResult<()> {
        let deps = mock_dependencies("secret", &[]);

        // A contract address
        let human_1 = HumanAddr::from("secret18vd8fpwxzck93qlwghaj6arh4p7c5n8978vsyg");
        let canon_1 = [
            59, 26, 116, 133, 198, 22, 44, 88, 131, 238, 69, 251, 45, 116, 119, 168, 125, 138, 76,
            229,
        ];
        let canon_1 = CanonicalAddr::from(canon_1.as_slice());
        // A user address
        let human_2 = HumanAddr::from("secret1ap26qrlp8mcq2pg6r47w43l0y8zkqm8a450s03");
        let canon_2 = [
            232, 85, 160, 15, 225, 62, 240, 5, 5, 26, 29, 124, 234, 199, 239, 33, 197, 96, 108, 253,
        ];
        let canon_2 = CanonicalAddr::from(canon_2.as_slice());

        for (human, canon) in [(human_1, canon_1), (human_2, canon_2)] {
            let response = deps.api.canonical_address(&human)?;
            assert_eq!(response, canon.clone());
            let response = deps.api.human_address(&canon)?;
            assert_eq!(response, human.clone());
        }

        Ok(())
    }
}

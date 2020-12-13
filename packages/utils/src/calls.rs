use serde::{de::DeserializeOwned, Serialize};

use cosmwasm_std::{
    to_binary, Coin, CosmosMsg, HumanAddr, Querier, QueryRequest, StdResult, Uint128, WasmMsg,
    WasmQuery,
};

use super::space_pad;

/// A data structure representing the instantiation message of a contract
///
/// This trait provides a method to create the CosmosMsg used to instantiate a contract
pub trait InitCallback: Serialize {
    /// pad the message to blocks of this size
    const BLOCK_SIZE: usize;

    /// Returns StdResult<CosmosMsg> used to instantiate the specified contract
    ///
    /// # Arguments
    ///
    /// * `label` - String holding the label for the new contract instance
    /// * `code_id` - code ID of the contract to be instantiated
    /// * `callback_code_hash` - String holding the code hash of the contract to be instantiated
    /// * `send_amount` - Optional Uint128 amount of native coin to send with instantiation message
    fn to_cosmos_msg(
        &self,
        label: String,
        code_id: u64,
        callback_code_hash: String,
        send_amount: Option<Uint128>,
    ) -> StdResult<CosmosMsg> {
        let mut msg = to_binary(self)?;
        // can not have 0 block size
        let padding = if Self::BLOCK_SIZE == 0 {
            1
        } else {
            Self::BLOCK_SIZE
        };
        space_pad(&mut msg.0, padding);
        let mut send = Vec::new();
        if let Some(amount) = send_amount {
            send.push(Coin {
                amount,
                denom: String::from("uscrt"),
            });
        }
        let init = WasmMsg::Instantiate {
            code_id,
            msg,
            callback_code_hash,
            send,
            label,
        };
        Ok(init.into())
    }
}

/// A data structure representing handle messages of a contract
///
/// This trait provides a method to create the CosmosMsg used to execute a handle method of a contract
pub trait HandleCallback: Serialize {
    /// pad the message to blocks of this size
    const BLOCK_SIZE: usize;

    /// Returns StdResult<CosmosMsg> used to execute the handle method of a contract
    ///
    /// # Arguments
    ///
    /// * `callback_code_hash` - String holding the code hash of the contract to be executed
    /// * `contract_addr` - address of the contract being called
    /// * `send_amount` - Optional Uint128 amount of native coin to send with the handle message
    fn to_cosmos_msg(
        &self,
        callback_code_hash: String,
        contract_addr: HumanAddr,
        send_amount: Option<Uint128>,
    ) -> StdResult<CosmosMsg> {
        let mut msg = to_binary(self)?;
        // can not have 0 block size
        let padding = if Self::BLOCK_SIZE == 0 {
            1
        } else {
            Self::BLOCK_SIZE
        };
        space_pad(&mut msg.0, padding);
        let mut send = Vec::new();
        if let Some(amount) = send_amount {
            send.push(Coin {
                amount,
                denom: String::from("uscrt"),
            });
        }
        let execute = WasmMsg::Execute {
            msg,
            contract_addr,
            callback_code_hash,
            send,
        };
        Ok(execute.into())
    }
}

/// A data structure representing query messages of a contract
///
/// This trait provides a method to query a contract
pub trait Query: Serialize {
    /// pad the message to blocks of this size
    const BLOCK_SIZE: usize;

    /// Returns StdResult<T>, where T is the response type that wraps the query answer
    ///
    /// # Arguments
    ///
    /// * `querier` - a reference to the Querier dependency of the querying contract
    /// * `callback_code_hash` - String holding the code hash of the contract to be queried
    /// * `contract_addr` - address of the contract being queried
    fn query<Q: Querier, T: DeserializeOwned>(
        &self,
        querier: &Q,
        callback_code_hash: String,
        contract_addr: HumanAddr,
    ) -> StdResult<T> {
        let mut msg = to_binary(self)?;
        // can not have 0 block size
        let padding = if Self::BLOCK_SIZE == 0 {
            1
        } else {
            Self::BLOCK_SIZE
        };
        space_pad(&mut msg.0, padding);
        querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr,
            callback_code_hash,
            msg,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    enum Foo {
        Var1 { f1: i8, f2: i8 },
        Var2 { f1: i8, f2: i8 },
    }

    // All you really need to do it make people give you the padding block size.
    impl HandleCallback for Foo {
        const BLOCK_SIZE: usize = 256;
    }

    #[test]
    fn test_callback_implementation_works() -> StdResult<()> {
        let address = HumanAddr("secret1xyzasdf".to_string());
        let hash = "asdf".to_string();
        let amount = Uint128(1234);

        let cosmos_message: CosmosMsg = Foo::Var1 { f1: 1, f2: 2 }.to_cosmos_msg(
            hash.clone(),
            address.clone(),
            Some(amount),
        )?;

        match cosmos_message {
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                callback_code_hash,
                msg,
                send,
            }) => {
                assert_eq!(contract_addr, address);
                assert_eq!(callback_code_hash, hash);
                let mut expected_msg = r#"{"Var1":{"f1":1,"f2":2}}"#.as_bytes().to_vec();
                space_pad(&mut expected_msg, 256);
                assert_eq!(msg.0, expected_msg);
                assert_eq!(send, vec![Coin::new(amount.0, "uscrt")])
            }
            other => panic!("unexpected CosmosMsg variant: {:?}", other),
        };

        Ok(())
    }
}

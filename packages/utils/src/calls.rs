use serde::{de::DeserializeOwned, Serialize};

use cosmwasm_std::{
    to_binary, Coin, CosmosMsg, CustomQuery, QuerierWrapper, QueryRequest, StdResult, Uint128,
    WasmMsg, WasmQuery,
};

use super::space_pad;

/// A trait marking types that define the instantiation message of a contract
///
/// This trait requires specifying a padding block size and provides a method to create the
/// CosmosMsg used to instantiate a contract
pub trait InitCallback: Serialize {
    /// pad the message to blocks of this size
    const BLOCK_SIZE: usize;

    /// Returns StdResult<CosmosMsg>
    ///
    /// Tries to convert the instance of the implementing type to a CosmosMsg that will trigger the
    /// instantiation of a contract.  The BLOCK_SIZE specified in the implementation is used when
    /// padding the message
    ///
    /// # Arguments
    ///
    /// * `label` - String holding the label for the new contract instance
    /// * `code_id` - code ID of the contract to be instantiated
    /// * `code_hash` - String holding the code hash of the contract to be instantiated
    /// * `funds_amount` - Optional Uint128 amount of native coin to send with instantiation message
    fn to_cosmos_msg(
        &self,
        label: String,
        code_id: u64,
        code_hash: String,
        funds_amount: Option<Uint128>,
    ) -> StdResult<CosmosMsg> {
        let mut msg = to_binary(self)?;
        // can not have 0 block size
        let padding = if Self::BLOCK_SIZE == 0 {
            1
        } else {
            Self::BLOCK_SIZE
        };
        space_pad(&mut msg.0, padding);
        let mut funds = Vec::new();
        if let Some(amount) = funds_amount {
            funds.push(Coin {
                amount,
                denom: String::from("uscrt"),
            });
        }
        let init = WasmMsg::Instantiate {
            code_id,
            msg,
            code_hash,
            funds,
            label,
        };
        Ok(init.into())
    }
}

/// A trait marking types that define the handle message(s) of a contract
///
/// This trait requires specifying a padding block size and provides a method to create the
/// CosmosMsg used to execute a handle method of a contract
pub trait HandleCallback: Serialize {
    /// pad the message to blocks of this size
    const BLOCK_SIZE: usize;

    /// Returns StdResult<CosmosMsg>
    ///
    /// Tries to convert the instance of the implementing type to a CosmosMsg that will trigger a
    /// handle function of a contract.  The BLOCK_SIZE specified in the implementation is used when
    /// padding the message
    ///
    /// # Arguments
    ///
    /// * `code_hash` - String holding the code hash of the contract to be executed
    /// * `contract_addr` - address of the contract being called
    /// * `funds_amount` - Optional Uint128 amount of native coin to send with the handle message
    fn to_cosmos_msg(
        &self,
        code_hash: String,
        contract_addr: String,
        funds_amount: Option<Uint128>,
    ) -> StdResult<CosmosMsg> {
        let mut msg = to_binary(self)?;
        // can not have 0 block size
        let padding = if Self::BLOCK_SIZE == 0 {
            1
        } else {
            Self::BLOCK_SIZE
        };
        space_pad(&mut msg.0, padding);
        let mut funds = Vec::new();
        if let Some(amount) = funds_amount {
            funds.push(Coin {
                amount,
                denom: String::from("uscrt"),
            });
        }
        let execute = WasmMsg::Execute {
            msg,
            contract_addr,
            code_hash,
            funds,
        };
        Ok(execute.into())
    }
}

/// A trait marking types that define the query message(s) of a contract
///
/// This trait requires specifying a padding block size and provides a method to query a contract
pub trait Query: Serialize {
    /// pad the message to blocks of this size
    const BLOCK_SIZE: usize;

    /// Returns StdResult<T>, where T is the type defining the query response
    ///
    /// Tries to query a contract and deserialize the query response.  The BLOCK_SIZE specified in the
    /// implementation is used when padding the message
    ///
    /// # Arguments
    ///
    /// * `querier` - a reference to the Querier dependency of the querying contract
    /// * `callback_code_hash` - String holding the code hash of the contract to be queried
    /// * `contract_addr` - address of the contract being queried
    fn query<'a, C: CustomQuery, T: DeserializeOwned>(
        &self,
        querier: QuerierWrapper<'a, C>,
        code_hash: String,
        contract_addr: String,
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
            code_hash,
            msg,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{
        to_vec, Binary, ContractResult, Empty, Querier, QuerierResult, SystemError, SystemResult,
    };
    use serde::Deserialize;

    #[derive(Serialize)]
    struct FooInit {
        pub f1: i8,
        pub f2: i8,
    }

    impl InitCallback for FooInit {
        const BLOCK_SIZE: usize = 256;
    }

    #[derive(Serialize)]
    enum FooHandle {
        Var1 { f1: i8, f2: i8 },
    }

    // All you really need to do is make people give you the padding block size.
    impl HandleCallback for FooHandle {
        const BLOCK_SIZE: usize = 256;
    }

    #[derive(Serialize)]
    enum FooQuery {
        Query1 { f1: i8, f2: i8 },
    }

    impl Query for FooQuery {
        const BLOCK_SIZE: usize = 256;
    }

    #[test]
    fn test_handle_callback_implementation_works() -> StdResult<()> {
        let address = "secret1xyzasdf".to_string();
        let hash = "asdf".to_string();
        let amount = Uint128::new(1234);

        let cosmos_message: CosmosMsg = FooHandle::Var1 { f1: 1, f2: 2 }.to_cosmos_msg(
            hash.clone(),
            address.clone(),
            Some(amount),
        )?;

        match cosmos_message {
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                code_hash,
                msg,
                funds,
            }) => {
                assert_eq!(contract_addr, address);
                assert_eq!(code_hash, hash);
                let mut expected_msg = r#"{"Var1":{"f1":1,"f2":2}}"#.as_bytes().to_vec();
                space_pad(&mut expected_msg, 256);
                assert_eq!(msg.0, expected_msg);
                assert_eq!(funds, vec![Coin::new(amount.u128(), "uscrt")])
            }
            other => panic!("unexpected CosmosMsg variant: {:?}", other),
        };

        Ok(())
    }

    #[test]
    fn test_init_callback_implementation_works() -> StdResult<()> {
        let lbl = "testlabel".to_string();
        let id = 17u64;
        let hash = "asdf".to_string();
        let amount = Uint128::new(1234);

        let cosmos_message: CosmosMsg =
            FooInit { f1: 1, f2: 2 }.to_cosmos_msg(lbl.clone(), id, hash.clone(), Some(amount))?;

        match cosmos_message {
            CosmosMsg::Wasm(WasmMsg::Instantiate {
                code_id,
                msg,
                code_hash,
                funds,
                label,
            }) => {
                assert_eq!(code_id, id);
                let mut expected_msg = r#"{"f1":1,"f2":2}"#.as_bytes().to_vec();
                space_pad(&mut expected_msg, 256);
                assert_eq!(msg.0, expected_msg);
                assert_eq!(code_hash, hash);
                assert_eq!(funds, vec![Coin::new(amount.u128(), "uscrt")]);
                assert_eq!(label, lbl)
            }
            other => panic!("unexpected CosmosMsg variant: {:?}", other),
        };

        Ok(())
    }

    #[test]
    fn test_query_works() -> StdResult<()> {
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct QueryResponse {
            bar1: i8,
            bar2: i8,
        }

        struct MyMockQuerier {}

        impl Querier for MyMockQuerier {
            fn raw_query(&self, request: &[u8]) -> QuerierResult {
                let mut expected_msg = r#"{"Query1":{"f1":1,"f2":2}}"#.as_bytes().to_vec();
                space_pad(&mut expected_msg, 256);
                let expected_request: QueryRequest<FooQuery> =
                    QueryRequest::Wasm(WasmQuery::Smart {
                        contract_addr: "secret1xyzasdf".to_string(),
                        code_hash: "asdf".to_string(),
                        msg: Binary(expected_msg),
                    });
                let test_req: &[u8] = &to_vec(&expected_request).unwrap();
                assert_eq!(request, test_req);
                let response = match to_binary(&QueryResponse { bar1: 1, bar2: 2 }) {
                    Ok(response) => ContractResult::Ok(response),
                    Err(_e) => return SystemResult::Err(SystemError::Unknown {}),
                };
                SystemResult::Ok(response)
            }
        }

        let querier = QuerierWrapper::<Empty>::new(&MyMockQuerier {});
        let address = "secret1xyzasdf".to_string();
        let hash = "asdf".to_string();

        let response: QueryResponse =
            FooQuery::Query1 { f1: 1, f2: 2 }.query(querier, hash, address)?;
        assert_eq!(response, QueryResponse { bar1: 1, bar2: 2 });

        Ok(())
    }
}

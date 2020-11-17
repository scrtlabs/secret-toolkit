use serde::Serialize;

use cosmwasm_std::{to_binary, Coin, CosmosMsg, HumanAddr, StdResult, Uint128, WasmMsg};

use super::space_pad;

pub trait Callback: Serialize {
    const BLOCK_SIZE: usize;

    fn to_cosmos_msg(
        &self,
        callback_code_hash: String,
        contract_addr: HumanAddr,
        send_amount: Option<Uint128>,
    ) -> StdResult<CosmosMsg> {
        let mut msg = to_binary(self)?;
        space_pad(&mut msg.0, Self::BLOCK_SIZE);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    enum Foo {
        Var1 { f1: i8, f2: i8 },
        Var2 { f1: i8, f2: i8 },
    }

    // All you really need to do it make people give you the padding block size.
    impl Callback for Foo {
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

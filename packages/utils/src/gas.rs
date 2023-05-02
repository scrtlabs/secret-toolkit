use cosmwasm_std::{Env, Deps, DepsMut, StdResult, StdError};

use secret_toolkit_crypto::{ContractPrng};
use secret_toolkit_crypto::RngCore;

/// randomize_gas_usage is a function that helps a contract set it's final gas usage to a random
/// amount.
///
/// For example, as a developer after testing I see that my contract uses between 19,500 and 40,300
/// gas, depending on inputs. In this case, I might want to set the final usage of gas to some random
/// number between 40,000 and 50,000 - that way regardless of even the most minute details, the gas
/// consumed by my contract will not leak information about the input.
///
/// `min_gas_used` - the final minimum amount of gas the contract will consume. If the contract already
/// used more than the minimum, this value will be ignored
/// `max_gas_to_add` - the function will generate a random number in the range <0, max_gas_to_add> and use that
/// value as extra gas to consume. Defaults to 10,000 if unspecified
///
/// Returns the total amount of gas used by the contract after execution
///
/// Will panic if env.block.random is unavailable, and return an error if either the check_gas or gas_evaporate
/// APIs return an error.
pub fn randomize_gas_usage(env: &Env, deps: &Deps, min_gas_used: Option<u32>, max_gas_to_add: Option<u32>) -> StdResult<u32> {

    let mut extra_to_target: u32 = 0;

    if let Some(target) = min_gas_used {
        let current_usage = deps.api.check_gas()?;

        if (current_usage as u32) < target {
            extra_to_target += target - (current_usage as u32)
        }
    }

    let mut rng = ContractPrng::from_env(env);

    let gas_to_consume = rng.next_u32() % max_gas_to_add.unwrap_or(10_000);

    deps.api.gas_evaporate(gas_to_consume)?;

    Ok(extra_to_target + gas_to_consume)
}


#[cfg(test)]
mod tests {

    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use crate::gas::randomize_gas_usage;

    #[test]
    pub fn test_randomize_gas_usage_no_params() {
        let env = mock_env();
        let deps = mock_dependencies();

        let gas_used = randomize_gas_usage(&env, &deps.as_ref(), None, None);

        assert_eq!(gas_used.is_ok(), true);

        assert!(gas_used.unwrap() < 10_000);
    }

    #[test]
    pub fn test_randomize_gas_usage_min_gas_used() {
        let env = mock_env();
        let deps = mock_dependencies();

        let gas_used = randomize_gas_usage(&env, &deps.as_ref(), Some(10_000), None);

        assert_eq!(gas_used.is_ok(), true);

        let value = gas_used.unwrap();
        assert!(value < 20_000);
        assert!(value >= 10_000);
    }

    #[test]
    pub fn test_randomize_gas_usage_max_gas_to_add() {
        let env = mock_env();
        let deps = mock_dependencies();

        let gas_used = randomize_gas_usage(&env, &deps.as_ref(), Some(10_000), Some(100_000));

        assert_eq!(gas_used.is_ok(), true);

        let value = gas_used.unwrap();
        assert!(value < 110_000);
        assert!(value >= 10_000);
    }
}
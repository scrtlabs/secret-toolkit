# Secret Contract Development Toolkit - Utility Tools

⚠️ This package is a sub-package of the `secret-toolkit` package. Please see its crate page for more context.

This package contains various uncategorized tools. It should be thought of
as the shed in your backyard where you put the stuff that doesn't belong
elsewhere. There isn't an overarching theme for the items in this package.

# Table of Contents
1. [Calls module](#calls-module)
2. [Feature Toggle module](#feature-toggle)

## Calls module
This module contains traits used to call another contract.  Do not forget to add the `use` statement for the traits you want.
```rust
use secret_toolkit::utils::{InitCallback, HandleCallback, Query};
```
Also, don't forget to add the toolkit dependency to your Cargo.toml

### Instantiating another contract
If you want to instantiate another contract, you should first copy/paste the InitMsg of that contract.  For example, if you wanted to create an instance of the counter contract at https://github.com/enigmampc/secret-template
```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CounterInitMsg {
    pub count: i32,
}

impl InitCallback for CounterInitMsg {
    const BLOCK_SIZE: usize = 256;
}
```
You would copy/paste its InitMsg, and rename it so that it does not conflict with the InitMsg you have defined for your own contract.  Then you would implement the `InitCallback` trait as above, setting the BLOCK_SIZE constant to the size of the blocks you want your instantiation message padded to.
```rust
let counter_init_msg = CounterInitMsg {
     count: 100 
};

let cosmos_msg = counter_init_msg.to_cosmos_msg(
    "new_contract_label".to_string(),
    123,
    "CODE_HASH_OF_CONTRACT_YOU_WANT_TO_INSTANTIATE".to_string(),
    None,
)?;

Ok(HandleResponse {
    messages: vec![cosmos_msg],
    log: vec![],
    data: None,
})
```
Next, in the init or handle function that will instantiate the other contract, you will create an instance of the CounterInitMsg, call its `to_cosmos_msg`, and place the resulting CosmosMsg in the `messages` Vec of the InitResponse or HandleResponse that your function is returning.  In this example, we are pretending that the code id of the counter contract is 123.  Also, in this example, you are not sending any SCRT with the InitMsg, but if you needed to send 1 SCRT, you would replace the None in the `to_cosmos_msg` call with `Some(Uint128(1000000))`.  The amount sent is in uscrt.  Any CosmosMsg placed in the `messages` Vec will be executed after your contract has finished its own processing.

### Calling a handle function of another contract
You should first copy/paste the specific HandleMsg(s) you want to call.  For example, if you wanted to reset the counter you instantiated above
```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum CounterHandleMsg {
    Reset { count: i32 },
}

impl HandleCallback for CounterHandleMsg {
    const BLOCK_SIZE: usize = 256;
}
```
You would copy/paste the Reset variant of its HandleMsg enum, and rename the enum so that it does not conflict with the HandleMsg enum you have defined for your own contract.  Then you would implement the `HandleCallback` trait as above, setting the BLOCK_SIZE constant to the size of the blocks you want your Reset message padded to.  If you need to call multiple different Handle messages, even if they are to different contracts, you can include all the Handle messages as variants in the same enum (you can not have two variants with the same name within the same enum, though).
```rust
let reset_msg = CounterHandleMsg::Reset {
    count: 200,
};

let cosmos_msg = reset_msg.to_cosmos_msg(
    "CODE_HASH_OF_CONTRACT_YOU_WANT_TO_EXECUTE".to_string(),
    HumanAddr("ADDRESS_OF_CONTRACT_YOU_ARE_CALLING".to_string()),
    None,
)?;

Ok(HandleResponse {
    messages: vec![cosmos_msg],
    log: vec![],
    data: None,
})
```
Next, in the init or handle function that will call the other contract, you will create an instance of the CounterHandleMsg::Reset variant, call its `to_cosmos_msg`, and place the resulting CosmosMsg in the `messages` Vec of the InitResponse or HandleResponse that your function is returning.  In this example, you are not sending any SCRT with the Reset message, but if you needed to send 1 SCRT, you would replace the None in the `to_cosmos_msg` call with `Some(Uint128(1000000))`.  The amount sent is in uscrt.  Any CosmosMsg placed in the `messages` Vec will be executed after your contract has finished its own processing.

### Querying another contract
You should first copy/paste the specific QueryMsg(s) you want to call.  For example, if you wanted to get the count of the counter you instantiated above
```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CounterQueryMsg {
    GetCount {},
}

impl Query for CounterQueryMsg {
    const BLOCK_SIZE: usize = 256;
}
```
You would copy/paste the GetCount variant of its QueryMsg enum, and rename the enum so that it does not conflict with the QueryMsg enum you have defined for your own contract.  Then you would implement the `Query` trait as above, setting the BLOCK_SIZE constant to the size of the blocks you want your query message padded to.  If you need to perform multiple different queries, even if they are to different contracts, you can include all the Query messages as variants in the same enum (you can not have two variants with the same name within the same enum, though).
```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CountResponse {
    pub count: i32,
}
```
Next, you will copy/paste the response of the query.  If the other contract defines its response to the query with a struct, you are good to go.

If, however, the other contract returns an enum variant, one approach is to copy the fields of the variant and place them in a struct.  Because an enum variant gets serialized with the name of the variant, you will then also want to create a wrapper struct whose only field has the name of the variant, and whose type is the struct you defined with the variant's fields.  For example, if you wanted to do a token_info query of the [SNIP20 reference implementation](https://github.com/enigmampc/snip20-reference-impl), I would recommend using the SNIP20 toolkit function, but just for the sake of example, let's say you forgot that toolkit existed.
```rust
#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub struct TokenInfo {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: Option<Uint128>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub struct TokenInfoResponse {
    pub token_info: TokenInfo,
}
```
You would copy the QueryAnswer::TokenInfo enum variant and create a TokenInfo struct with those fields.  You should make all those fields public if you need to access them.  Then you would create the TokenInfoResponse wrapper struct, which has only one field whose name is the name of the QueryAnswer variant in snake case (token_info).  As a reminder, you only need to do this to properly deserialize the response if it was defined as an enum in the other contract.

Now to perform the query
```rust
let get_count = CounterQueryMsg::GetCount {};
let count_response: CountResponse = get_count.query(
    &deps.querier,
    "CODE_HASH_OF_CONTRACT_YOU_WANT_TO_QUERY".to_string(),
    HumanAddr("ADDRESS_OF_CONTRACT_YOU_ARE_QUERYING".to_string()),
)?;
```
You create an instance of the CounterQueryMsg::GetCount variant, and call its `query` function, returning its value to a variable of the response type.  If you were doing a token_info query, you would write `let token_info_resp: TokenInfoResponse = ...`.  You MUST use explicit type annotation here.

## Feature Toggle

This module implements feature toggles for your contract. The main motivation behind it is to enable pausing/unpausing certain operations rather than pausing/unpausing the contract entirely, while providing you with helper functions that will reduce your code to a minimum.

The feature toggles are designed to be flexible, so you can choose whether to put entire messages under a toggle or just certain code sections, etc.

### Initializing Features

Normally you'd want to initialize the features in the `init()` function:
```rust
pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    FeatureToggle::init_features(
        &mut deps.storage,
        vec![
            FeatureStatus {
                feature: Features::Feature1,
                status: Default::default(),
            },
            FeatureStatus {
                feature: Features::Feature2,
                status: Default::default(),
            },
        ],
        vec![env.message.sender], // Can put more than one pauser
    )?;
}
```

The feature field in `FeatureStatus` can be anything, as long as it's implementing `serde::Serialize`.
In this example it's:
```rust
#[derive(Serialize)]
pub enum Features {
    Feature1,
    Feature2,
}
```

For the `status` field, you should use the built-in `FeatureToggle::Status` enum: 
```rust
#[derive(Serialize, Debug, Deserialize, Clone, JsonSchema, PartialEq)]
pub enum Status {
    NotPaused,
    Paused,
}
```
The defult value of `Status` is `Status::NotPaused`.

### Put a toggle on a message

Putting a toggle on a message (or any code section of your choosing) is as easy as calling `FeatureToggle::require_not_paused()`. For example if we have a `Redeem` message in our contract, and we initialized the feature as `Features::Redeem`:
```rust
fn redeem<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    amount: Option<u128>,
) -> StdResult<HandleResponse> {
    FeatureToggle::require_not_paused(&deps.storage, vec![Features::Redeem])?;
    
    // Continue with function's operation
}
```
If the status of the `Features::Redeem` feature is `Paused`, the contract will error out and stop operation.

### Pause/unpause a feature

Firstly, we will need to add `Pause` and `Unpause` messages in our `HandleMsg` enum. We can simply use `FeatureToggle::FeatureToggleHandleMsg` - it's an enum that contains default messages that `FeatureToggle` also has default implementation for:
```rust
pub enum HandleMsg {
    // Contract messages
    Redeem {
        amount: Option<Uint128>,
    },
    Etc {}, //..

    // Feature toggle
    Features(FeatureToggleHandleMsg),
}
```

The `FeatureToggle` struct contains a default implementation for triggering (pausing/unpausing) a feature, so you can just call it from your `handle()` function:
```rust
pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::Redeem { amount } => redeem(deps, env, amount),
        HandleMsg::Etc {} => etc(deps, env),
        HandleMsg::Features(m) => match m {
            FeatureToggleHandleMsg::Pause { features } => FeatureToggle::handle_pause(deps, env, features),
            FeatureToggleHandleMsg::Unpause { features } => FeatureToggle::handle_unpause(deps, env, features),
        },
    }
}
```

Note: `FeatureToggle::pause()` and `FeatureToggle::unpause()` requires `env.message.sender` to be a pauser!

### Adding/removing pausers

Similarly to the section above, add `FeatureToggleHandleMsg` to your `HandleMsg`.

Note: you should only add `Features(FeatureToggleHandleMsg)` to the `HandleMsg` enum once, and it'll add all the supported messages.

`FeatureToggle` provides with default implementation for these too, but you can wrap it with your own logic like requiring the caller to be admin, etc.:
```rust
pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    // This is the same `match` clause from the section above
    match msg {
        HandleMsg::Redeem { amount } => redeem(deps, env, amount),
        HandleMsg::Features(m) => match m {
            // `Stop` and `Resume` go here too
            FeatureToggleHandleMsg::SetPauser { address } => set_pauser(deps, env, address),
            FeatureToggleHandleMsg::RemovePauser { address } => remove_pauser(deps, env, address),
        },
    }
}

fn set_pauser<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    address: HumanAddr,
) -> StdResult<HandleResponse> {
    let admin = get_admin()?;
    if admin != env.message.sender {
        return Err(StdError::unauthorized());
    }

    FeatureToggle::handle_set_pauser(deps, env, address)
}

fn remove_pauser<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    address: HumanAddr,
) -> StdResult<HandleResponse> {
    let admin = get_admin()?;
    if admin != env.message.sender {
        return Err(StdError::unauthorized());
    }

    FeatureToggle::handle_remove_pauser(deps, env, address)
}
```

Note: `set_pauser` and `remove_pauser` are permissionless by default.

### Overriding the default implementation

If you don't like the default implementation or want to override it for any other reason (for example, using a different storage namespace), you can do that by defining your own struct and implement `FeatureToggleTrait` for it:
```rust
struct TrollFeatureToggle {}

impl FeatureToggleTrait for TrollFeatureToggle {
    // This is mandatory
    const STORAGE_KEY: &'static [u8] = b"custom_and_super_cool_key";

    // This is optional
    fn pause<S: Storage, T: Serialize>(storage: &mut S, features: Vec<T>) -> StdResult<()> {
        for f in features {
            Self::set_feature_status(storage, &f, Status::NotPaused)?;
        }

        Ok(())
    }

    // This is optional
    fn unpause<S: Storage, T: Serialize>(storage: &mut S, features: Vec<T>) -> StdResult<()> {
        for f in features {
            Self::set_feature_status(storage, &f, Status::Paused)?;
        }

        Ok(())
    }
}
```

### Queries

Similarly to `FeatureToggleHandleMsg`, query messages (and default implementations) are also provided:
```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FeatureToggleQueryMsg<T: Serialize + DeserializeOwned> {
    #[serde(bound = "")] // don't ask
    Status {
        features: Vec<T>,
    },
    IsPauser {
        address: HumanAddr,
    },
}
```

You can use them in your `query()` the same way you used `FeatureToggleHandleMsg`.

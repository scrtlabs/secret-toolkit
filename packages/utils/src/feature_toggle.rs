use cosmwasm_std::{
    to_binary, Api, Env, Extern, HandleResponse, HumanAddr, Querier, QueryResult, ReadonlyStorage,
    StdError, StdResult, Storage,
};
use cosmwasm_storage::{Bucket, ReadonlyBucket};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

static PREFIX_FEATURE_TOGGLE: &[u8] = b"featuretoggle";
static PREFIX_FEATURES: &[u8] = b"features";
static PREFIX_PAUSERS: &[u8] = b"pausers";

pub struct FeatureToggle {}

impl FeatureToggle {
    pub fn init_features<S: Storage>(
        storage: &mut S,
        features: Vec<String>,
        initial_state: Option<Vec<FeatureStatus>>,
        pausers: Vec<HumanAddr>,
    ) -> StdResult<()> {
        let initial_state =
            initial_state.unwrap_or_else(|| vec![FeatureStatus::default(); features.len()]);

        if initial_state.len() != features.len() {
            return Err(StdError::generic_err("feature toggle: can't initialize features! `features` and `initial_state` should be of equal length"));
        }

        for (feature, state) in features.iter().zip(initial_state.iter()) {
            Self::_set_feature_status(storage, feature.clone(), state.clone())?;
        }

        for p in pausers {
            Self::_set_pauser(storage, p)?;
        }

        Ok(())
    }

    pub fn require_resumed<S: Storage>(storage: &S, features: Vec<String>) -> StdResult<()> {
        for f in features {
            let status = Self::_get_feature_status(storage, f.clone())?;
            match status {
                None => {
                    return Err(StdError::generic_err(format!(
                        "feature toggle: unknown feature '{}'",
                        f
                    )))
                }
                Some(s) => match s {
                    FeatureStatus::Resumed => {}
                    FeatureStatus::Stopped => {
                        return Err(StdError::generic_err(format!(
                            "feature toggle: feature '{}' is stopped",
                            f
                        )));
                    }
                },
            }
        }

        Ok(())
    }

    pub fn handle_stop<S: Storage, A: Api, Q: Querier>(
        deps: &mut Extern<S, A, Q>,
        env: Env,
        features: Vec<String>,
    ) -> StdResult<HandleResponse> {
        if Self::_get_pauser(&deps.storage, env.message.sender)?.is_none() {
            return Err(StdError::unauthorized());
        }

        Self::_stop(&mut deps.storage, features)?;

        Ok(HandleResponse {
            messages: vec![],
            log: vec![],
            data: Some(to_binary(&HandleAnswer::Stop {
                status: ResponseStatus::Success,
            })?),
        })
    }

    pub fn handle_resume<S: Storage, A: Api, Q: Querier>(
        deps: &mut Extern<S, A, Q>,
        env: Env,
        features: Vec<String>,
    ) -> StdResult<HandleResponse> {
        if Self::_get_pauser(&deps.storage, env.message.sender)?.is_none() {
            return Err(StdError::unauthorized());
        }

        Self::_resume(&mut deps.storage, features)?;

        Ok(HandleResponse {
            messages: vec![],
            log: vec![],
            data: Some(to_binary(&HandleAnswer::Resume {
                status: ResponseStatus::Success,
            })?),
        })
    }

    pub fn handle_set_pauser<S: Storage, A: Api, Q: Querier>(
        deps: &mut Extern<S, A, Q>,
        _env: Env,
        address: HumanAddr,
    ) -> StdResult<HandleResponse> {
        Self::_set_pauser(&mut deps.storage, address)?;

        Ok(HandleResponse {
            messages: vec![],
            log: vec![],
            data: Some(to_binary(&HandleAnswer::SetPauser {
                status: ResponseStatus::Success,
            })?),
        })
    }

    pub fn handle_remove_pauser<S: Storage, A: Api, Q: Querier>(
        deps: &mut Extern<S, A, Q>,
        _env: Env,
        address: HumanAddr,
    ) -> StdResult<HandleResponse> {
        Self::_remove_pauser(&mut deps.storage, address);

        Ok(HandleResponse {
            messages: vec![],
            log: vec![],
            data: Some(to_binary(&HandleAnswer::RemovePauser {
                status: ResponseStatus::Success,
            })?),
        })
    }

    pub fn query_status<S: Storage, A: Api, Q: Querier>(
        deps: &Extern<S, A, Q>,
        features: Vec<String>,
    ) -> QueryResult {
        let mut status = vec![];
        for f in features {
            match Self::_get_feature_status(&deps.storage, f.clone())? {
                None => {
                    return Err(StdError::generic_err(format!(
                        "invalid feature: {} does not exist",
                        f
                    )))
                }
                Some(s) => status.push(_FeatureStatus {
                    feature: f,
                    status: s,
                }),
            }
        }

        to_binary(&FeatureToggleQueryAnswer::Status { features: status })
    }

    pub fn query_is_pauser<S: Storage, A: Api, Q: Querier>(
        deps: &Extern<S, A, Q>,
        address: HumanAddr,
    ) -> QueryResult {
        let is_pauser = Self::_get_pauser(&deps.storage, address)?.is_some();

        to_binary(&FeatureToggleQueryAnswer::IsPauser { is_pauser })
    }

    fn _stop<S: Storage>(storage: &mut S, features: Vec<String>) -> StdResult<()> {
        for f in features {
            Self::_set_feature_status(storage, f, FeatureStatus::Stopped)?;
        }

        Ok(())
    }

    fn _resume<S: Storage>(storage: &mut S, features: Vec<String>) -> StdResult<()> {
        for f in features {
            Self::_set_feature_status(storage, f, FeatureStatus::Resumed)?;
        }

        Ok(())
    }

    fn _get_pauser<S: ReadonlyStorage>(storage: &S, key: HumanAddr) -> StdResult<Option<u8>> {
        let feature_store =
            ReadonlyBucket::multilevel(&[PREFIX_FEATURE_TOGGLE, PREFIX_PAUSERS], storage);
        feature_store.may_load(key.0.as_bytes())
    }

    fn _set_pauser<S: Storage>(storage: &mut S, key: HumanAddr) -> StdResult<()> {
        let mut feature_store =
            Bucket::multilevel(&[PREFIX_FEATURE_TOGGLE, PREFIX_PAUSERS], storage);
        feature_store.save(key.0.as_bytes(), &1_u8 /* value is insignificant */)
    }

    fn _remove_pauser<S: Storage>(storage: &mut S, key: HumanAddr) {
        let mut feature_store: Bucket<S, u8> =
            Bucket::multilevel(&[PREFIX_FEATURE_TOGGLE, PREFIX_PAUSERS], storage);
        feature_store.remove(key.0.as_bytes())
    }

    fn _get_feature_status<S: ReadonlyStorage>(
        storage: &S,
        key: String,
    ) -> StdResult<Option<FeatureStatus>> {
        let feature_store =
            ReadonlyBucket::multilevel(&[PREFIX_FEATURE_TOGGLE, PREFIX_FEATURES], storage);
        feature_store.may_load(key.as_bytes())
    }

    fn _set_feature_status<S: Storage>(
        storage: &mut S,
        key: String,
        item: FeatureStatus,
    ) -> StdResult<()> {
        let mut feature_store =
            Bucket::multilevel(&[PREFIX_FEATURE_TOGGLE, PREFIX_FEATURES], storage);
        feature_store.save(key.as_bytes(), &item)
    }
}

#[derive(Serialize, Debug, Deserialize, Clone, JsonSchema, PartialEq)]
pub enum FeatureStatus {
    Resumed,
    Stopped,
}

impl Default for FeatureStatus {
    fn default() -> Self {
        FeatureStatus::Resumed
    }
}

#[derive(Serialize, Debug, Deserialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FeatureToggleMsg {
    Stop { features: Vec<String> },
    Resume { features: Vec<String> },
    SetPauser { address: HumanAddr },
    RemovePauser { address: HumanAddr },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
enum ResponseStatus {
    Success,
    Failure,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum HandleAnswer {
    Stop { status: ResponseStatus },
    Resume { status: ResponseStatus },
    SetPauser { status: ResponseStatus },
    RemovePauser { status: ResponseStatus },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FeatureToggleQueryMsg {
    Status {},
    IsPauser { address: HumanAddr },
}

#[derive(Serialize, Deserialize, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
enum FeatureToggleQueryAnswer {
    Status { features: Vec<_FeatureStatus> },
    IsPauser { is_pauser: bool },
}

#[derive(Serialize, Deserialize, JsonSchema, Debug)]
struct _FeatureStatus {
    feature: String,
    status: FeatureStatus,
}

#[cfg(test)]
mod tests {
    use crate::feature_toggle::{FeatureStatus, FeatureToggle};
    use cosmwasm_std::testing::MockStorage;
    use cosmwasm_std::{HumanAddr, MemoryStorage, StdResult};

    fn init_features(storage: &mut MemoryStorage) -> StdResult<()> {
        FeatureToggle::init_features(
            storage,
            vec![
                "Feature1".to_string(),
                "Feature2".to_string(),
                "Feature3".to_string(),
            ],
            Some(vec![
                FeatureStatus::Resumed,
                FeatureStatus::Resumed,
                FeatureStatus::Stopped,
            ]),
            vec![HumanAddr("alice".to_string())],
        )
    }

    #[test]
    fn test_init_works() -> StdResult<()> {
        let mut storage = MockStorage::new();
        init_features(&mut storage)?;

        assert_eq!(
            FeatureToggle::get_feature_status(&storage, "Feature1".to_string())?,
            Some(FeatureStatus::Resumed)
        );
        assert_eq!(
            FeatureToggle::get_feature_status(&storage, "Feature2".to_string())?,
            Some(FeatureStatus::Resumed)
        );
        assert_eq!(
            FeatureToggle::get_feature_status(&storage, "Feature3".to_string())?,
            Some(FeatureStatus::Stopped)
        );
        assert_eq!(
            FeatureToggle::get_feature_status(&storage, "Feature4".to_string())?,
            None
        );

        assert_eq!(
            FeatureToggle::_get_pauser(&storage, HumanAddr("alice".to_string()))?,
            Some(1_u8)
        );
        assert_eq!(
            FeatureToggle::_get_pauser(&storage, HumanAddr("bob".to_string()))?,
            None
        );

        Ok(())
    }

    #[test]
    fn test_init_different_lengths() -> StdResult<()> {
        let mut storage = MockStorage::new();
        assert!(FeatureToggle::init_features(
            &mut storage,
            vec![
                "Feature1".to_string(),
                "Feature2".to_string(),
                "Feature3".to_string(),
                "Feature4".to_string(),
            ],
            Some(vec![
                FeatureStatus::Resumed,
                FeatureStatus::Resumed,
                FeatureStatus::Stopped,
            ]),
            vec![HumanAddr("alice".to_string())],
        )
        .is_err());

        Ok(())
    }

    #[test]
    fn test_resume() -> StdResult<()> {
        let mut storage = MockStorage::new();
        init_features(&mut storage)?;

        FeatureToggle::_resume(&mut storage, vec!["Feature3".to_string()])?;
        assert_eq!(
            FeatureToggle::get_feature_status(&storage, "Feature3".to_string())?,
            Some(FeatureStatus::Resumed)
        );

        Ok(())
    }

    #[test]
    fn test_stop() -> StdResult<()> {
        let mut storage = MockStorage::new();
        init_features(&mut storage)?;

        FeatureToggle::_stop(&mut storage, vec!["Feature1".to_string()])?;
        assert_eq!(
            FeatureToggle::get_feature_status(&storage, "Feature1".to_string())?,
            Some(FeatureStatus::Stopped)
        );

        Ok(())
    }

    #[test]
    fn test_require_resumed() -> StdResult<()> {
        let mut storage = MockStorage::new();
        init_features(&mut storage)?;

        assert!(FeatureToggle::require_resumed(&storage, vec!["Feature1".to_string()]).is_ok());
        assert!(FeatureToggle::require_resumed(&storage, vec!["Feature3".to_string()]).is_err());

        Ok(())
    }

    #[test]
    fn test_add_remove_pausers() -> StdResult<()> {
        let mut storage = MockStorage::new();
        init_features(&mut storage)?;

        let bob = HumanAddr("bob".to_string());

        FeatureToggle::_set_pauser(&mut storage, bob.clone())?;
        assert!(FeatureToggle::_get_pauser(&storage, bob.clone())?.is_some());

        FeatureToggle::_remove_pauser(&mut storage, bob.clone());
        assert!(FeatureToggle::_get_pauser(&storage, bob.clone())?.is_none());

        Ok(())
    }
}

use cosmwasm_std::{
    to_binary, Api, Env, Extern, HandleResponse, HumanAddr, Querier, QueryResult, ReadonlyStorage,
    StdError, StdResult, Storage,
};
use cosmwasm_storage::{Bucket, ReadonlyBucket};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

const PREFIX_FEATURES: &[u8] = b"features";
const PREFIX_PAUSERS: &[u8] = b"pausers";

pub struct FeatureToggle;

impl FeatureToggleStore for FeatureToggle {
    const STORAGE_KEY: &'static [u8] = b"feature_toggle";
}

pub trait FeatureToggleStore {
    const STORAGE_KEY: &'static [u8];

    fn init_features<S: Storage, T: Serialize>(
        storage: &mut S,
        feature_statuses: Vec<FeatureStatus<T>>,
        pausers: Vec<HumanAddr>,
    ) -> StdResult<()> {
        for feature_status in feature_statuses {
            Self::set_feature_status(storage, feature_status.feature, feature_status.status)?;
        }

        for p in pausers {
            Self::set_pauser(storage, p)?;
        }

        Ok(())
    }

    fn pause<S: Storage, T: Serialize>(storage: &mut S, features: Vec<T>) -> StdResult<()> {
        for f in features {
            Self::set_feature_status(storage, f, Status::Paused)?;
        }

        Ok(())
    }

    fn unpause<S: Storage, T: Serialize>(storage: &mut S, features: Vec<T>) -> StdResult<()> {
        for f in features {
            Self::set_feature_status(storage, f, Status::NotPaused)?;
        }

        Ok(())
    }

    fn get_pauser<S: ReadonlyStorage>(storage: &S, key: HumanAddr) -> StdResult<Option<u8>> {
        let feature_store =
            ReadonlyBucket::multilevel(&[Self::STORAGE_KEY, PREFIX_PAUSERS], storage);
        feature_store.may_load(key.0.as_bytes())
    }

    fn set_pauser<S: Storage>(storage: &mut S, key: HumanAddr) -> StdResult<()> {
        let mut feature_store = Bucket::multilevel(&[Self::STORAGE_KEY, PREFIX_PAUSERS], storage);
        feature_store.save(key.0.as_bytes(), &1_u8 /* value is insignificant */)
    }

    fn remove_pauser<S: Storage>(storage: &mut S, key: HumanAddr) {
        let mut feature_store: Bucket<S, u8> =
            Bucket::multilevel(&[Self::STORAGE_KEY, PREFIX_PAUSERS], storage);
        feature_store.remove(key.0.as_bytes())
    }

    fn get_feature_status<S: ReadonlyStorage, T: Serialize>(
        storage: &S,
        key: T,
    ) -> StdResult<Option<Status>> {
        let feature_store =
            ReadonlyBucket::multilevel(&[Self::STORAGE_KEY, PREFIX_FEATURES], storage);
        feature_store.may_load(&cosmwasm_std::to_vec(&key)?)
    }

    fn set_feature_status<S: Storage, T: Serialize>(
        storage: &mut S,
        key: T,
        item: Status,
    ) -> StdResult<()> {
        let mut feature_store = Bucket::multilevel(&[Self::STORAGE_KEY, PREFIX_FEATURES], storage);
        feature_store.save(&cosmwasm_std::to_vec(&key)?, &item)
    }
}

impl FeatureToggle {
    pub fn require_not_paused<S: Storage, T: Serialize + Display + Clone>(
        storage: &S,
        features: Vec<T>,
    ) -> StdResult<()> {
        for feature in features {
            let status = Self::get_feature_status(storage, feature.clone())?;
            match status {
                None => {
                    return Err(StdError::generic_err(format!(
                        "feature toggle: unknown feature '{}'",
                        feature
                    )))
                }
                Some(s) => match s {
                    Status::NotPaused => {}
                    Status::Paused => {
                        return Err(StdError::generic_err(format!(
                            "feature toggle: feature '{}' is paused",
                            feature
                        )));
                    }
                },
            }
        }

        Ok(())
    }

    pub fn handle_pause<S: Storage, A: Api, Q: Querier, T: Serialize>(
        deps: &mut Extern<S, A, Q>,
        env: Env,
        features: Vec<T>,
    ) -> StdResult<HandleResponse> {
        if Self::get_pauser(&deps.storage, env.message.sender)?.is_none() {
            return Err(StdError::unauthorized());
        }

        Self::pause(&mut deps.storage, features)?;

        Ok(HandleResponse {
            messages: vec![],
            log: vec![],
            data: Some(to_binary(&HandleAnswer::Pause {
                status: ResponseStatus::Success,
            })?),
        })
    }

    pub fn handle_unpause<S: Storage, A: Api, Q: Querier, T: Serialize>(
        deps: &mut Extern<S, A, Q>,
        env: Env,
        features: Vec<T>,
    ) -> StdResult<HandleResponse> {
        if Self::get_pauser(&deps.storage, env.message.sender)?.is_none() {
            return Err(StdError::unauthorized());
        }

        Self::unpause(&mut deps.storage, features)?;

        Ok(HandleResponse {
            messages: vec![],
            log: vec![],
            data: Some(to_binary(&HandleAnswer::Unpause {
                status: ResponseStatus::Success,
            })?),
        })
    }

    pub fn handle_set_pauser<S: Storage, A: Api, Q: Querier>(
        deps: &mut Extern<S, A, Q>,
        _env: Env,
        address: HumanAddr,
    ) -> StdResult<HandleResponse> {
        Self::set_pauser(&mut deps.storage, address)?;

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
        Self::remove_pauser(&mut deps.storage, address);

        Ok(HandleResponse {
            messages: vec![],
            log: vec![],
            data: Some(to_binary(&HandleAnswer::RemovePauser {
                status: ResponseStatus::Success,
            })?),
        })
    }

    pub fn query_status<S: Storage, A: Api, Q: Querier, T: Serialize + Display + Clone>(
        deps: &Extern<S, A, Q>,
        features: Vec<T>,
    ) -> QueryResult {
        let mut status = Vec::with_capacity(features.len());
        for feature in features {
            match Self::get_feature_status(&deps.storage, feature.clone())? {
                None => {
                    return Err(StdError::generic_err(format!(
                        "invalid feature: {} does not exist",
                        feature
                    )))
                }
                Some(s) => status.push(FeatureStatus { feature, status: s }),
            }
        }

        to_binary(&FeatureToggleQueryAnswer::Status { features: status })
    }

    pub fn query_is_pauser<S: Storage, A: Api, Q: Querier>(
        deps: &Extern<S, A, Q>,
        address: HumanAddr,
    ) -> QueryResult {
        let is_pauser = Self::get_pauser(&deps.storage, address)?.is_some();

        to_binary(&FeatureToggleQueryAnswer::<()>::IsPauser { is_pauser })
    }
}

#[derive(Serialize, Debug, Deserialize, Clone, JsonSchema, PartialEq)]
pub enum Status {
    NotPaused,
    Paused,
}

impl Default for Status {
    fn default() -> Self {
        Status::NotPaused
    }
}

#[derive(Serialize, Debug, Deserialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FeatureToggleMsg {
    Pause { features: Vec<String> },
    Unpause { features: Vec<String> },
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
    Pause { status: ResponseStatus },
    Unpause { status: ResponseStatus },
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
enum FeatureToggleQueryAnswer<T: Serialize> {
    Status { features: Vec<FeatureStatus<T>> },
    IsPauser { is_pauser: bool },
}

#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub struct FeatureStatus<T: Serialize> {
    feature: T,
    status: Status,
}

#[cfg(test)]
mod tests {
    use crate::feature_toggle::{FeatureStatus, FeatureToggle, FeatureToggleStore, Status};
    use cosmwasm_std::testing::MockStorage;
    use cosmwasm_std::{HumanAddr, MemoryStorage, StdResult};

    fn init_features(storage: &mut MemoryStorage) -> StdResult<()> {
        FeatureToggle::init_features(
            storage,
            vec![
                FeatureStatus {
                    feature: "Feature1".to_string(),
                    status: Status::NotPaused,
                },
                FeatureStatus {
                    feature: "Feature2".to_string(),
                    status: Status::NotPaused,
                },
                FeatureStatus {
                    feature: "Feature3".to_string(),
                    status: Status::Paused,
                },
            ],
            vec![HumanAddr("alice".to_string())],
        )
    }

    #[test]
    fn test_init_works() -> StdResult<()> {
        let mut storage = MockStorage::new();
        init_features(&mut storage)?;

        assert_eq!(
            FeatureToggle::get_feature_status(&storage, "Feature1".to_string())?,
            Some(Status::NotPaused)
        );
        assert_eq!(
            FeatureToggle::get_feature_status(&storage, "Feature2".to_string())?,
            Some(Status::NotPaused)
        );
        assert_eq!(
            FeatureToggle::get_feature_status(&storage, "Feature3".to_string())?,
            Some(Status::Paused)
        );
        assert_eq!(
            FeatureToggle::get_feature_status(&storage, "Feature4".to_string())?,
            None
        );

        assert_eq!(
            FeatureToggle::get_pauser(&storage, HumanAddr("alice".to_string()))?,
            Some(1_u8)
        );
        assert_eq!(
            FeatureToggle::get_pauser(&storage, HumanAddr("bob".to_string()))?,
            None
        );

        Ok(())
    }

    #[test]
    fn test_unpause() -> StdResult<()> {
        let mut storage = MockStorage::new();
        init_features(&mut storage)?;

        FeatureToggle::unpause(&mut storage, vec!["Feature3".to_string()])?;
        assert_eq!(
            FeatureToggle::get_feature_status(&storage, "Feature3".to_string())?,
            Some(Status::NotPaused)
        );

        Ok(())
    }

    #[test]
    fn test_pause() -> StdResult<()> {
        let mut storage = MockStorage::new();
        init_features(&mut storage)?;

        FeatureToggle::pause(&mut storage, vec!["Feature1".to_string()])?;
        assert_eq!(
            FeatureToggle::get_feature_status(&storage, "Feature1".to_string())?,
            Some(Status::Paused)
        );

        Ok(())
    }

    #[test]
    fn test_require_not_paused() -> StdResult<()> {
        let mut storage = MockStorage::new();
        init_features(&mut storage)?;

        assert!(FeatureToggle::require_not_paused(&storage, vec!["Feature1".to_string()]).is_ok());
        assert!(FeatureToggle::require_not_paused(&storage, vec!["Feature3".to_string()]).is_err());

        Ok(())
    }

    #[test]
    fn test_add_remove_pausers() -> StdResult<()> {
        let mut storage = MockStorage::new();
        init_features(&mut storage)?;

        let bob = HumanAddr("bob".to_string());

        FeatureToggle::set_pauser(&mut storage, bob.clone())?;
        assert!(FeatureToggle::get_pauser(&storage, bob.clone())?.is_some());

        FeatureToggle::remove_pauser(&mut storage, bob.clone());
        assert!(FeatureToggle::get_pauser(&storage, bob.clone())?.is_none());

        Ok(())
    }
}

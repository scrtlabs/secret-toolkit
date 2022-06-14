use cosmwasm_std::{
    to_binary, to_vec, Api, Env, Extern, HandleResponse, HandleResult, HumanAddr, Querier,
    QueryResult, ReadonlyStorage, StdError, StdResult, Storage,
};
use cosmwasm_storage::{Bucket, ReadonlyBucket};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

const PREFIX_FEATURES: &[u8] = b"features";
const PREFIX_PAUSERS: &[u8] = b"pausers";

pub struct FeatureToggle;

impl FeatureToggleTrait for FeatureToggle {
    const STORAGE_KEY: &'static [u8] = b"feature_toggle";
}

pub trait FeatureToggleTrait {
    const STORAGE_KEY: &'static [u8];

    fn init_features<S: Storage, T: Serialize>(
        storage: &mut S,
        feature_statuses: Vec<FeatureStatus<T>>,
        pausers: Vec<HumanAddr>,
    ) -> StdResult<()> {
        for fs in feature_statuses {
            Self::set_feature_status(storage, &fs.feature, fs.status)?;
        }

        for p in pausers {
            Self::set_pauser(storage, &p)?;
        }

        Ok(())
    }

    fn require_not_paused<S: Storage, T: Serialize>(
        storage: &S,
        features: Vec<T>,
    ) -> StdResult<()> {
        for feature in features {
            let status = Self::get_feature_status(storage, &feature)?;
            match status {
                None => {
                    return Err(StdError::generic_err(format!(
                        "feature toggle: unknown feature '{}'",
                        String::from_utf8_lossy(&to_vec(&feature)?)
                    )))
                }
                Some(s) => match s {
                    Status::NotPaused => {}
                    Status::Paused => {
                        return Err(StdError::generic_err(format!(
                            "feature toggle: feature '{}' is paused",
                            String::from_utf8_lossy(&to_vec(&feature)?)
                        )));
                    }
                },
            }
        }

        Ok(())
    }

    fn pause<S: Storage, T: Serialize>(storage: &mut S, features: Vec<T>) -> StdResult<()> {
        for f in features {
            Self::set_feature_status(storage, &f, Status::Paused)?;
        }

        Ok(())
    }

    fn unpause<S: Storage, T: Serialize>(storage: &mut S, features: Vec<T>) -> StdResult<()> {
        for f in features {
            Self::set_feature_status(storage, &f, Status::NotPaused)?;
        }

        Ok(())
    }

    fn is_pauser<S: ReadonlyStorage>(storage: &S, key: &HumanAddr) -> StdResult<bool> {
        let feature_store: ReadonlyBucket<S, bool> =
            ReadonlyBucket::multilevel(&[Self::STORAGE_KEY, PREFIX_PAUSERS], storage);
        feature_store
            .may_load(key.0.as_bytes())
            .map(|p| p.is_some())
    }

    fn set_pauser<S: Storage>(storage: &mut S, key: &HumanAddr) -> StdResult<()> {
        let mut feature_store = Bucket::multilevel(&[Self::STORAGE_KEY, PREFIX_PAUSERS], storage);
        feature_store.save(key.0.as_bytes(), &true /* value is insignificant */)
    }

    fn remove_pauser<S: Storage>(storage: &mut S, key: &HumanAddr) {
        let mut feature_store: Bucket<S, bool> =
            Bucket::multilevel(&[Self::STORAGE_KEY, PREFIX_PAUSERS], storage);
        feature_store.remove(key.0.as_bytes())
    }

    fn get_feature_status<S: ReadonlyStorage, T: Serialize>(
        storage: &S,
        key: &T,
    ) -> StdResult<Option<Status>> {
        let feature_store =
            ReadonlyBucket::multilevel(&[Self::STORAGE_KEY, PREFIX_FEATURES], storage);
        feature_store.may_load(&cosmwasm_std::to_vec(&key)?)
    }

    fn set_feature_status<S: Storage, T: Serialize>(
        storage: &mut S,
        key: &T,
        item: Status,
    ) -> StdResult<()> {
        let mut feature_store = Bucket::multilevel(&[Self::STORAGE_KEY, PREFIX_FEATURES], storage);
        feature_store.save(&cosmwasm_std::to_vec(&key)?, &item)
    }

    fn handle_pause<S: Storage, A: Api, Q: Querier, T: Serialize>(
        deps: &mut Extern<S, A, Q>,
        env: &Env,
        features: Vec<T>,
    ) -> HandleResult {
        if !Self::is_pauser(&deps.storage, &env.message.sender)? {
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

    fn handle_unpause<S: Storage, A: Api, Q: Querier, T: Serialize>(
        deps: &mut Extern<S, A, Q>,
        env: &Env,
        features: Vec<T>,
    ) -> HandleResult {
        if !Self::is_pauser(&deps.storage, &env.message.sender)? {
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

    fn handle_set_pauser<S: Storage, A: Api, Q: Querier>(
        deps: &mut Extern<S, A, Q>,
        _env: &Env,
        address: HumanAddr,
    ) -> HandleResult {
        Self::set_pauser(&mut deps.storage, &address)?;

        Ok(HandleResponse {
            messages: vec![],
            log: vec![],
            data: Some(to_binary(&HandleAnswer::SetPauser {
                status: ResponseStatus::Success,
            })?),
        })
    }

    fn handle_remove_pauser<S: Storage, A: Api, Q: Querier>(
        deps: &mut Extern<S, A, Q>,
        _env: &Env,
        address: HumanAddr,
    ) -> HandleResult {
        Self::remove_pauser(&mut deps.storage, &address);

        Ok(HandleResponse {
            messages: vec![],
            log: vec![],
            data: Some(to_binary(&HandleAnswer::RemovePauser {
                status: ResponseStatus::Success,
            })?),
        })
    }

    fn query_status<S: Storage, A: Api, Q: Querier, T: Serialize>(
        deps: &Extern<S, A, Q>,
        features: Vec<T>,
    ) -> QueryResult {
        let mut status = Vec::with_capacity(features.len());
        for feature in features {
            match Self::get_feature_status(&deps.storage, &feature)? {
                None => {
                    return Err(StdError::generic_err(format!(
                        "invalid feature: {} does not exist",
                        String::from_utf8_lossy(&to_vec(&feature)?)
                    )))
                }
                Some(s) => status.push(FeatureStatus { feature, status: s }),
            }
        }

        to_binary(&FeatureToggleQueryAnswer::Status { features: status })
    }

    fn query_is_pauser<S: Storage, A: Api, Q: Querier>(
        deps: &Extern<S, A, Q>,
        address: HumanAddr,
    ) -> QueryResult {
        let is_pauser = Self::is_pauser(&deps.storage, &address)?;

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FeatureToggleHandleMsg<T: Serialize + DeserializeOwned> {
    #[serde(bound = "")]
    Pause {
        features: Vec<T>,
    },
    #[serde(bound = "")]
    Unpause {
        features: Vec<T>,
    },
    SetPauser {
        address: HumanAddr,
    },
    RemovePauser {
        address: HumanAddr,
    },
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
pub enum FeatureToggleQueryMsg<T: Serialize + DeserializeOwned> {
    #[serde(bound = "")]
    Status {
        features: Vec<T>,
    },
    IsPauser {
        address: HumanAddr,
    },
}

#[derive(Serialize, Deserialize, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
enum FeatureToggleQueryAnswer<T: Serialize> {
    Status { features: Vec<FeatureStatus<T>> },
    IsPauser { is_pauser: bool },
}

#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub struct FeatureStatus<T: Serialize> {
    pub feature: T,
    pub status: Status,
}

#[cfg(test)]
mod tests {
    use crate::feature_toggle::{
        FeatureStatus, FeatureToggle, FeatureToggleHandleMsg, FeatureToggleQueryMsg,
        FeatureToggleTrait, HandleAnswer, ResponseStatus, Status,
    };
    use cosmwasm_std::testing::{mock_dependencies, mock_env, MockStorage};
    use cosmwasm_std::{from_binary, HumanAddr, MemoryStorage, StdError, StdResult};

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
            FeatureToggle::get_feature_status(&storage, &"Feature1".to_string())?,
            Some(Status::NotPaused)
        );
        assert_eq!(
            FeatureToggle::get_feature_status(&storage, &"Feature2".to_string())?,
            Some(Status::NotPaused)
        );
        assert_eq!(
            FeatureToggle::get_feature_status(&storage, &"Feature3".to_string())?,
            Some(Status::Paused)
        );
        assert_eq!(
            FeatureToggle::get_feature_status(&storage, &"Feature4".to_string())?,
            None
        );

        assert_eq!(
            FeatureToggle::is_pauser(&storage, &HumanAddr("alice".to_string()))?,
            true
        );
        assert_eq!(
            FeatureToggle::is_pauser(&storage, &HumanAddr("bob".to_string()))?,
            false
        );

        Ok(())
    }

    #[test]
    fn test_unpause() -> StdResult<()> {
        let mut storage = MockStorage::new();
        init_features(&mut storage)?;

        FeatureToggle::unpause(&mut storage, vec!["Feature3".to_string()])?;
        assert_eq!(
            FeatureToggle::get_feature_status(&storage, &"Feature3".to_string())?,
            Some(Status::NotPaused)
        );

        Ok(())
    }

    #[test]
    fn test_handle_unpause() -> StdResult<()> {
        let mut deps = mock_dependencies(20, &[]);
        init_features(&mut deps.storage)?;

        let env = mock_env("non-pauser", &[]);
        let error = FeatureToggle::handle_unpause(&mut deps, &env, vec!["Feature3".to_string()]);
        assert_eq!(error, Err(StdError::unauthorized()));

        let env = mock_env("alice", &[]);
        let response =
            FeatureToggle::handle_unpause(&mut deps, &env, vec!["Feature3".to_string()])?;
        let answer: HandleAnswer = from_binary(&response.data.unwrap())?;

        assert_eq!(
            answer,
            HandleAnswer::Unpause {
                status: ResponseStatus::Success,
            }
        );
        Ok(())
    }

    #[test]
    fn test_pause() -> StdResult<()> {
        let mut storage = MockStorage::new();
        init_features(&mut storage)?;

        FeatureToggle::pause(&mut storage, vec!["Feature1".to_string()])?;
        assert_eq!(
            FeatureToggle::get_feature_status(&storage, &"Feature1".to_string())?,
            Some(Status::Paused)
        );

        Ok(())
    }

    #[test]
    fn test_handle_pause() -> StdResult<()> {
        let mut deps = mock_dependencies(20, &[]);
        init_features(&mut deps.storage)?;

        let env = mock_env("non-pauser", &[]);
        let error = FeatureToggle::handle_pause(&mut deps, &env, vec!["Feature2".to_string()]);
        assert_eq!(error, Err(StdError::unauthorized()));

        let env = mock_env("alice", &[]);
        let response = FeatureToggle::handle_pause(&mut deps, &env, vec!["Feature2".to_string()])?;
        let answer: HandleAnswer = from_binary(&response.data.unwrap())?;

        assert_eq!(
            answer,
            HandleAnswer::Pause {
                status: ResponseStatus::Success,
            }
        );
        Ok(())
    }

    #[test]
    fn test_require_not_paused() -> StdResult<()> {
        let mut storage = MockStorage::new();
        init_features(&mut storage)?;

        assert!(
            FeatureToggle::require_not_paused(&storage, vec!["Feature1".to_string()]).is_ok(),
            "{:?}",
            FeatureToggle::require_not_paused(&storage, vec!["Feature1".to_string()])
        );
        assert!(
            FeatureToggle::require_not_paused(&storage, vec!["Feature3".to_string()]).is_err(),
            "{:?}",
            FeatureToggle::require_not_paused(&storage, vec!["Feature3".to_string()])
        );

        Ok(())
    }

    #[test]
    fn test_add_remove_pausers() -> StdResult<()> {
        let mut storage = MockStorage::new();
        init_features(&mut storage)?;

        let bob = HumanAddr("bob".to_string());

        FeatureToggle::set_pauser(&mut storage, &bob)?;
        assert!(
            FeatureToggle::is_pauser(&storage, &bob)?,
            "{:?}",
            FeatureToggle::is_pauser(&storage, &bob)
        );

        FeatureToggle::remove_pauser(&mut storage, &bob);
        assert!(
            !FeatureToggle::is_pauser(&storage, &bob)?,
            "{:?}",
            FeatureToggle::is_pauser(&storage, &bob)
        );

        Ok(())
    }

    #[test]
    fn test_deserialize_messages() {
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        #[serde(rename_all = "snake_case")]
        enum Features {
            Var1,
            Var2,
        }

        let handle_msg = b"{\"pause\":{\"features\":[\"var1\",\"var2\"]}}";
        let query_msg = b"{\"status\":{\"features\": [\"var1\"]}}";
        let query_msg_invalid = b"{\"status\":{\"features\": [\"var3\"]}}";

        let parsed: FeatureToggleHandleMsg<Features> =
            cosmwasm_std::from_slice(handle_msg).unwrap();
        assert_eq!(
            parsed,
            FeatureToggleHandleMsg::Pause {
                features: vec![Features::Var1, Features::Var2]
            }
        );
        let parsed: FeatureToggleQueryMsg<Features> = cosmwasm_std::from_slice(query_msg).unwrap();
        assert_eq!(
            parsed,
            FeatureToggleQueryMsg::Status {
                features: vec![Features::Var1]
            }
        );
        let parsed: StdResult<FeatureToggleQueryMsg<Features>> =
            cosmwasm_std::from_slice(query_msg_invalid);
        assert!(parsed.is_err());
    }
}

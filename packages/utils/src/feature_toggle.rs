use cosmwasm_std::{
    to_binary, to_vec, Addr, Binary, Deps, DepsMut, MessageInfo, Response, StdError, StdResult,
    Storage,
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

    fn init_features<T: Serialize>(
        storage: &mut dyn Storage,
        feature_statuses: Vec<FeatureStatus<T>>,
        pausers: Vec<Addr>,
    ) -> StdResult<()> {
        for fs in feature_statuses {
            Self::set_feature_status(storage, &fs.feature, fs.status)?;
        }

        for p in pausers {
            Self::set_pauser(storage, &p)?;
        }

        Ok(())
    }

    fn require_not_paused<T: Serialize>(storage: &dyn Storage, features: Vec<T>) -> StdResult<()> {
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

    fn pause<T: Serialize>(storage: &mut dyn Storage, features: Vec<T>) -> StdResult<()> {
        for f in features {
            Self::set_feature_status(storage, &f, Status::Paused)?;
        }

        Ok(())
    }

    fn unpause<T: Serialize>(storage: &mut dyn Storage, features: Vec<T>) -> StdResult<()> {
        for f in features {
            Self::set_feature_status(storage, &f, Status::NotPaused)?;
        }

        Ok(())
    }

    fn is_pauser(storage: &dyn Storage, key: &Addr) -> StdResult<bool> {
        let feature_store: ReadonlyBucket<bool> =
            ReadonlyBucket::multilevel(storage, &[Self::STORAGE_KEY, PREFIX_PAUSERS]);
        feature_store.may_load(key.as_bytes()).map(|p| p.is_some())
    }

    fn set_pauser(storage: &mut dyn Storage, key: &Addr) -> StdResult<()> {
        let mut feature_store = Bucket::multilevel(storage, &[Self::STORAGE_KEY, PREFIX_PAUSERS]);
        feature_store.save(key.as_bytes(), &true /* value is insignificant */)
    }

    fn remove_pauser(storage: &mut dyn Storage, key: &Addr) {
        let mut feature_store: Bucket<bool> =
            Bucket::multilevel(storage, &[Self::STORAGE_KEY, PREFIX_PAUSERS]);
        feature_store.remove(key.as_bytes())
    }

    fn get_feature_status<T: Serialize>(
        storage: &dyn Storage,
        key: &T,
    ) -> StdResult<Option<Status>> {
        let feature_store =
            ReadonlyBucket::multilevel(storage, &[Self::STORAGE_KEY, PREFIX_FEATURES]);
        feature_store.may_load(&cosmwasm_std::to_vec(&key)?)
    }

    fn set_feature_status<T: Serialize>(
        storage: &mut dyn Storage,
        key: &T,
        item: Status,
    ) -> StdResult<()> {
        let mut feature_store = Bucket::multilevel(storage, &[Self::STORAGE_KEY, PREFIX_FEATURES]);
        feature_store.save(&cosmwasm_std::to_vec(&key)?, &item)
    }

    fn handle_pause<T: Serialize>(
        deps: DepsMut,
        info: &MessageInfo,
        features: Vec<T>,
    ) -> StdResult<Response> {
        if !Self::is_pauser(deps.storage, &info.sender)? {
            return Err(StdError::generic_err("unauthorized"));
        }

        Self::pause(deps.storage, features)?;

        Ok(Response::new().set_data(to_binary(&HandleAnswer::Pause {
            status: ResponseStatus::Success,
        })?))
    }

    fn handle_unpause<T: Serialize>(
        deps: DepsMut,
        info: &MessageInfo,
        features: Vec<T>,
    ) -> StdResult<Response> {
        if !Self::is_pauser(deps.storage, &info.sender)? {
            return Err(StdError::generic_err("unauthorized"));
        }

        Self::unpause(deps.storage, features)?;

        Ok(Response::new().set_data(to_binary(&HandleAnswer::Unpause {
            status: ResponseStatus::Success,
        })?))
    }

    fn handle_set_pauser(deps: DepsMut, address: Addr) -> StdResult<Response> {
        Self::set_pauser(deps.storage, &address)?;

        Ok(
            Response::new().set_data(to_binary(&HandleAnswer::SetPauser {
                status: ResponseStatus::Success,
            })?),
        )
    }

    fn handle_remove_pauser(deps: DepsMut, address: Addr) -> StdResult<Response> {
        Self::remove_pauser(deps.storage, &address);

        Ok(
            Response::new().set_data(to_binary(&HandleAnswer::RemovePauser {
                status: ResponseStatus::Success,
            })?),
        )
    }

    fn query_status<T: Serialize>(deps: Deps, features: Vec<T>) -> StdResult<Binary> {
        let mut status = Vec::with_capacity(features.len());
        for feature in features {
            match Self::get_feature_status(deps.storage, &feature)? {
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

    fn query_is_pauser(deps: Deps, address: Addr) -> StdResult<Binary> {
        let is_pauser = Self::is_pauser(deps.storage, &address)?;

        to_binary(&FeatureToggleQueryAnswer::<()>::IsPauser { is_pauser })
    }
}

#[derive(Serialize, Debug, Deserialize, Clone, JsonSchema, PartialEq, Eq)]
pub enum Status {
    NotPaused,
    Paused,
}

impl Default for Status {
    fn default() -> Self {
        Status::NotPaused
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
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
        address: String,
    },
    RemovePauser {
        address: String,
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FeatureToggleQueryMsg<T: Serialize + DeserializeOwned> {
    #[serde(bound = "")]
    Status {
        features: Vec<T>,
    },
    IsPauser {
        address: String,
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
    use cosmwasm_std::testing::{mock_dependencies, mock_info, MockStorage};
    use cosmwasm_std::{from_binary, Addr, MemoryStorage, StdError, StdResult};

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
            vec![Addr::unchecked("alice".to_string())],
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
            FeatureToggle::is_pauser(&storage, &Addr::unchecked("alice".to_string()))?,
            true
        );
        assert_eq!(
            FeatureToggle::is_pauser(&storage, &Addr::unchecked("bob".to_string()))?,
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
        let mut deps = mock_dependencies();
        init_features(&mut deps.storage)?;

        let info = mock_info("non-pauser", &[]);
        let error =
            FeatureToggle::handle_unpause(deps.as_mut(), &info, vec!["Feature3".to_string()]);
        assert_eq!(error, Err(StdError::generic_err("unauthorized")));

        let info = mock_info("alice", &[]);
        let response =
            FeatureToggle::handle_unpause(deps.as_mut(), &info, vec!["Feature3".to_string()])?;
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
        let mut deps = mock_dependencies();
        init_features(&mut deps.storage)?;

        let info = mock_info("non-pauser", &[]);
        let error = FeatureToggle::handle_pause(deps.as_mut(), &info, vec!["Feature2".to_string()]);
        assert_eq!(error, Err(StdError::generic_err("unauthorized")));

        let info = mock_info("alice", &[]);
        let response =
            FeatureToggle::handle_pause(deps.as_mut(), &info, vec!["Feature2".to_string()])?;
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

        let bob = Addr::unchecked("bob".to_string());

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

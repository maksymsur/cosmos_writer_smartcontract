#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, GetAdminResponse, GetWriteResponse, InstantiateMsg, QueryMsg};
use crate::state::{State, STATE};

// basic info about smartcontract
const CONTRACT_NAME: &str = "test_cosmos_writer";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {
        data: msg.data,
        admins: vec![info.sender.clone()],
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("admin", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Write { data } => execute::update(deps, info, data),
        ExecuteMsg::AddAdmin { admin } => execute::add_admin(deps, info, admin),
        ExecuteMsg::RemoveAdmin { admin } => execute::remove_admin(deps, info, admin),
    }
}

pub mod execute {
    use super::*;

    pub fn update(
        deps: DepsMut,
        info: MessageInfo,
        data: Vec<u8>,
    ) -> Result<Response, ContractError> {
        STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
            if !state.admins.contains(&info.sender) {
                return Err(ContractError::Unauthorized {});
            }
            state.data = data;
            Ok(state)
        })?;
        Ok(Response::new().add_attribute("action", "update"))
    }

    pub fn add_admin(
        deps: DepsMut,
        info: MessageInfo,
        admin: String,
    ) -> Result<Response, ContractError> {
        STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
            if !state.admins.contains(&info.sender) {
                return Err(ContractError::Unauthorized {});
            }
            let new_admin = deps.api.addr_validate(&admin)?;
            if !state.admins.contains(&new_admin) {
                state.admins.push(new_admin);
            }
            Ok(state)
        })?;
        Ok(Response::new()
            .add_attribute("action", "add_admin")
            .add_attribute("new_admin", admin))
    }

    pub fn remove_admin(
        deps: DepsMut,
        info: MessageInfo,
        admin: String,
    ) -> Result<Response, ContractError> {
        STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
            if !state.admins.contains(&info.sender) {
                return Err(ContractError::Unauthorized {});
            }
            let remove_admin = deps.api.addr_validate(&admin)?;
            state.admins.retain(|x| x != &remove_admin); // Remove the specified admin from the list via inplace op
            Ok(state)
        })?;
        Ok(Response::new()
            .add_attribute("action", "remove_admin")
            .add_attribute("removed_admin", admin))
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetData {} => to_binary(&query::data(deps)?), // instantiated for tests purely
        QueryMsg::GetAdmins {} => to_binary(&query::admins(deps)?),
    }
}

pub mod query {
    use super::*;

    /// Retrieving written data
    pub fn data(deps: Deps) -> StdResult<GetWriteResponse> {
        let state = STATE.load(deps.storage)?;
        Ok(GetWriteResponse { data: state.data })
    }

    /// Retrieving a list of admins
    pub fn admins(deps: Deps) -> StdResult<GetAdminResponse> {
        let state = STATE.load(deps.storage)?;
        Ok(GetAdminResponse {
            admins: state.admins,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary, Addr};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg { data: vec![17] };
        let info = mock_info("creator", &coins(1000, "token"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetData {}).unwrap();
        let value: GetWriteResponse = from_binary(&res).unwrap();
        assert_eq!(vec![17], value.data);
    }

    #[test]
    fn unauthorized_write_attempt() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg { data: vec![17] };
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // only owner can write data, thus, here we shall get an error on any unauthorized attempt to write
        let info = mock_info("anyone", &coins(2, "token"));
        let msg = ExecuteMsg::Write { data: vec![18] };
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();

        // data shall be unchanged
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetData {}).unwrap();
        let value: GetWriteResponse = from_binary(&res).unwrap();
        assert_eq!(vec![17], value.data);
    }

    #[test]
    fn allowed_write() {
        // setting up a test env
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg { data: vec![17] };
        let info = mock_info("creator", &coins(2, "token"));
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // testing that unauthorised `new_user` cannot write data
        let unauth_info = mock_info("anyone", &coins(2, "token"));
        let msg = ExecuteMsg::Write { data: vec![18] };
        let res = execute(deps.as_mut(), mock_env(), unauth_info, msg);
        match res {
            Err(ContractError::Unauthorized {}) => {}
            _ => panic!("Must return unauthorized error"),
        }

        // testing that only the authorised `creator` can write data
        let auth_info = mock_info("creator", &coins(2, "token"));
        let msg = ExecuteMsg::Write { data: vec![19] };
        execute(deps.as_mut(), mock_env(), auth_info, msg).unwrap();

        // checking the result that should now be vec![19]
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetData {}).unwrap();
        let value: GetWriteResponse = from_binary(&res).unwrap();
        assert_eq!(vec![19], value.data);
    }

    #[test]
    fn add_admins() {
        // setting up a test env
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg { data: vec![17] };
        let info = mock_info("creator", &coins(2, "token"));
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // testing that unauthorised `new_user` cannot write data
        let unauth_info = mock_info("new_user", &coins(2, "token"));
        let msg = ExecuteMsg::Write { data: vec![18] };
        let res = execute(deps.as_mut(), mock_env(), unauth_info, msg);
        match res {
            Err(ContractError::Unauthorized {}) => {}
            _ => panic!("Must return unauthorized error"),
        }

        // adding `new_user` to admins
        let auth_info = mock_info("creator", &coins(2, "token"));
        let msg = ExecuteMsg::AddAdmin {
            admin: "new_user".to_string(),
        };
        execute(deps.as_mut(), mock_env(), auth_info, msg).unwrap();

        // testing that `new_user` can write data now and overwrite vec![17] with vec![18]
        let auth_info = mock_info("new_user", &coins(2, "token"));
        let msg = ExecuteMsg::Write { data: vec![18] };
        execute(deps.as_mut(), mock_env(), auth_info, msg).unwrap();

        // checking the result that should now be vec![18]
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetData {}).unwrap();
        let value: GetWriteResponse = from_binary(&res).unwrap();
        assert_eq!(vec![18], value.data);
    }

    #[test]
    fn list_and_remove_admins() {
        // setting up a test env
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg { data: vec![17] };
        let info = mock_info("creator", &coins(2, "token"));
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // adding `new_user` to admins
        let auth_info = mock_info("creator", &coins(2, "token"));
        let msg = ExecuteMsg::AddAdmin {
            admin: "new_user".to_string(),
        };
        execute(deps.as_mut(), mock_env(), auth_info.clone(), msg).unwrap();

        // getting a list of admins
        let msg = QueryMsg::GetAdmins {};
        let res = query(deps.as_ref(), mock_env(), msg).unwrap();
        let list: GetAdminResponse = from_binary(&res).unwrap();
        let expected_admins = vec![Addr::unchecked("creator"), Addr::unchecked("new_user")];
        assert_eq!(list.admins, expected_admins);
        dbg!(list.admins); // print the list of admins before deletion

        // removing `new_user` admin and getting the updated list of admins
        let msg = ExecuteMsg::RemoveAdmin {
            admin: "new_user".to_string(),
        };
        execute(deps.as_mut(), mock_env(), auth_info, msg).unwrap();
        let msg = QueryMsg::GetAdmins {};
        let res = query(deps.as_ref(), mock_env(), msg).unwrap();
        let list: GetAdminResponse = from_binary(&res).unwrap();
        let expected_admins = vec![Addr::unchecked("creator")];
        assert_eq!(list.admins, expected_admins);
        dbg!(list.admins);
    }
}

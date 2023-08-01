/*
!!! EXPERIMENTAL !!!
Multitest is a design to simulate a blockchain environment in pure Rust.
This allows us to run unit tests that involve contract -> contract, and contract -> bank interactions.
This is not intended to be a full blockchain app but to simulate the Cosmos SDK x/wasm module close enough
to gain confidence in multi-contract deployements before testing them on a live blockchain.
DESIGN DOC: https://github.com/CosmWasm/cw-multi-test/blob/main/DESIGN.md
*/

#[cfg(test)]
mod helpers {
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    use cosmwasm_std::{to_binary, Addr, CosmosMsg, StdResult, WasmMsg, WasmQuery};

    use crate::msg::{ExecuteMsg, QueryMsg};

    /// CwTemplateContract is a wrapper around Addr that provides a lot of helpers
    /// for working with this.
    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
    pub struct CwTemplateContract(pub Addr);

    impl CwTemplateContract {
        pub fn addr(&self) -> Addr {
            self.0.clone()
        }

        pub fn call<T: Into<ExecuteMsg>>(&self, msg: T) -> StdResult<CosmosMsg> {
            let msg = to_binary(&msg.into())?;
            Ok(WasmMsg::Execute {
                contract_addr: self.addr().into(),
                msg,
                funds: vec![],
            }
            .into())
        }

        #[allow(dead_code)]
        /// Get admins list
        pub fn get_admins<T: Into<QueryMsg>>(&self, msg: T) -> StdResult<WasmQuery> {
            let msg = to_binary(&msg.into())?;
            let res = WasmQuery::Smart {
                contract_addr: self.addr().into(),
                msg,
            };

            Ok(res)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::integration_tests::helpers::CwTemplateContract;
    use crate::msg::InstantiateMsg;
    use cosmwasm_std::{Addr, Coin, Empty, Uint128};
    use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};

    pub fn contract_template() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        );
        Box::new(contract)
    }

    const USER: &str = "kujira1myl4t0y5eq3vjahjfm27re76xdr9zda4xerzd9";
    const ADMIN: &str = "kujira19n9ts2xpz5dz2a03808yjyj40d9e46ss8fgz2h";
    const NATIVE_DENOM: &str = "ukuji";

    fn mock_app() -> App {
        AppBuilder::new().build(|router, _, storage| {
            router
                .bank
                .init_balance(
                    storage,
                    &Addr::unchecked(USER),
                    vec![Coin {
                        denom: NATIVE_DENOM.to_string(),
                        amount: Uint128::new(1),
                    }],
                )
                .unwrap();
        })
    }

    fn proper_instantiate() -> (App, CwTemplateContract) {
        let mut app = mock_app();
        let cw_template_id = app.store_code(contract_template());

        let msg = InstantiateMsg { data: vec![17] };
        let cw_template_contract_addr = app
            .instantiate_contract(
                cw_template_id,
                Addr::unchecked(ADMIN),
                &msg,
                &[],
                "test",
                None,
            )
            .unwrap();

        let cw_template_contract = CwTemplateContract(cw_template_contract_addr);

        (app, cw_template_contract)
    }

    mod write {
        use super::*;

        use crate::msg::{ExecuteMsg, GetAdminResponse, QueryMsg};

        #[test]
        fn allowed_write() {
            let (mut app, cw_template_contract) = proper_instantiate();

            let msg = ExecuteMsg::Write { data: vec![17] };
            let cosmos_msg = cw_template_contract.call(msg).unwrap();
            let res = app.execute(Addr::unchecked(ADMIN), cosmos_msg).unwrap();
            dbg!(res);
        }

        #[test]
        fn unauthorised_write() {
            let (mut app, cw_template_contract) = proper_instantiate();

            let msg = ExecuteMsg::Write { data: vec![17, 18] };
            let cosmos_msg = cw_template_contract.call(msg).unwrap();
            app.execute(Addr::unchecked(USER), cosmos_msg).unwrap_err();
        }

        #[test]
        fn add_admin_and_write() {
            let (mut app, cw_template_contract) = proper_instantiate();

            let msg = ExecuteMsg::AddAdmin {
                admin: USER.to_owned(),
            };
            let cosmos_msg = cw_template_contract.call(msg).unwrap();
            app.execute(Addr::unchecked(ADMIN), cosmos_msg).unwrap();

            let msg = ExecuteMsg::Write { data: vec![17, 18] };
            let cosmos_msg = cw_template_contract.call(msg).unwrap();
            app.execute(Addr::unchecked(USER), cosmos_msg).unwrap();
        }

        #[test]
        fn add_and_remove_admin() {
            let (mut app, cw_template_contract) = proper_instantiate();

            let msg = ExecuteMsg::AddAdmin {
                admin: USER.to_owned(),
            };
            let cosmos_msg = cw_template_contract.call(msg).unwrap();
            let res = app.execute(Addr::unchecked(ADMIN), cosmos_msg).unwrap();
            dbg!(res);

            let msg = QueryMsg::GetAdmins {};
            let admins_list: GetAdminResponse = app
                .wrap()
                .query_wasm_smart(cw_template_contract.addr(), &msg)
                .unwrap();

            dbg!(admins_list.clone());

            let expected_admins = vec![Addr::unchecked(ADMIN), Addr::unchecked(USER)];
            assert_eq!(admins_list.admins, expected_admins);

            let msg = ExecuteMsg::RemoveAdmin {
                admin: USER.to_owned(),
            };

            let cosmos_msg = cw_template_contract.call(msg).unwrap();
            app.execute(Addr::unchecked(ADMIN), cosmos_msg).unwrap();

            let msg = QueryMsg::GetAdmins {};
            let admins_list: GetAdminResponse = app
                .wrap()
                .query_wasm_smart(cw_template_contract.addr(), &msg)
                .unwrap();

            dbg!(admins_list.clone());

            let expected_admins = vec![Addr::unchecked(ADMIN)];
            assert_eq!(admins_list.admins, expected_admins);
        }
    }
}

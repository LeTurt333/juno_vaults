use crate::error::*;
use crate::state::*;
use chrono::{Datelike, NaiveDateTime, Timelike};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_binary, Addr, BankMsg, CosmosMsg, Empty, StdError, StdResult, WasmMsg};
use cw20::{Balance, Cw20ExecuteMsg};
use cw721::Cw721ExecuteMsg;

pub fn send_tokens_cosmos(to: &Addr, balance: &GenericBalance) -> StdResult<Vec<CosmosMsg>> {
    let native_balance = &balance.native;
    let mut msgs: Vec<CosmosMsg> = if native_balance.is_empty() {
        vec![]
    } else {
        vec![CosmosMsg::from(BankMsg::Send {
            to_address: to.into(),
            amount: native_balance.to_vec(),
        })]
    };

    let cw20_balance = &balance.cw20;
    let cw20_msgs: StdResult<Vec<_>> = cw20_balance
        .iter()
        .map(|c| {
            // Only works if recipient is User Address, doesn't work for DAO / Contracts
            let msg = Cw20ExecuteMsg::Transfer {
                recipient: to.into(),
                amount: c.amount,
            };
            let exec = CosmosMsg::from(WasmMsg::Execute {
                contract_addr: c.address.to_string(),
                msg: to_binary(&msg)?,
                funds: vec![],
            });
            Ok(exec)
        })
        .collect();

    msgs.extend(cw20_msgs?);

    let nft_balance = &balance.nfts;
    let nft_msgs: StdResult<Vec<CosmosMsg<Empty>>> = nft_balance
        .iter()
        .map(|n| {
            let msg = Cw721ExecuteMsg::TransferNft {
                recipient: to.into(),
                token_id: n.token_id.clone(),
            };
            let exec = CosmosMsg::from(WasmMsg::Execute {
                contract_addr: n.contract_address.to_string(),
                msg: to_binary(&msg)?,
                funds: vec![],
            });
            Ok(exec)
        })
        .collect();

    msgs.extend(nft_msgs?);

    Ok(msgs)
}

pub fn is_balance_whitelisted(balance: &Balance, config: &Config) -> Result<(), ContractError> {
    let wl_native_denoms: Vec<_> =
        config.whitelist_native.iter().map(|double| double.1.clone()).collect();

    let wl_cw20_addys: Vec<_> =
        config.whitelist_cw20.iter().map(|double2| double2.1.clone()).collect();

    match balance {
        Balance::Native(natives_sent_in) => {
            let bool_vec: Vec<bool> = natives_sent_in
                .0
                .iter()
                .map(|native| wl_native_denoms.contains(&native.denom))
                .collect();
            if bool_vec.contains(&false) {
                return Err(ContractError::NotWhitelist {
                    which: "fail 1 Native".to_string(),
                });
            }
        }
        Balance::Cw20(cw20) => {
            if !wl_cw20_addys.contains(&cw20.address) {
                return Err(ContractError::NotWhitelist {
                    which: "fail 2 Cw20".to_string(),
                });
            }
        }
    }

    Ok(())
}

pub fn is_genericbalance_whitelisted(
    genericbalance: &GenericBalance,
    config: &Config,
) -> Result<(), ContractError> {
    let wl_native_denoms: Vec<_> =
        config.whitelist_native.iter().map(|double| double.1.clone()).collect();

    if !genericbalance.native.is_empty() {
        for native in genericbalance.native.clone() {
            if !wl_native_denoms.contains(&native.denom) {
                return Err(ContractError::NotWhitelist {
                    which: native.denom,
                });
            };
        }
    }

    let wl_cw20_addys: Vec<_> =
        config.whitelist_cw20.iter().map(|double2| double2.1.clone()).collect();

    if !genericbalance.cw20.is_empty() {
        for cw20coin in genericbalance.cw20.clone() {
            if !wl_cw20_addys.contains(&cw20coin.address) {
                return Err(ContractError::NotWhitelist {
                    which: cw20coin.address.into_string(),
                });
            };
        }
    }

    let wl_nft_addys: Vec<_> =
        config.whitelist_nft.iter().map(|double3| double3.1.clone()).collect();

    if !genericbalance.nfts.is_empty() {
        for nft in genericbalance.nfts.clone() {
            if !wl_nft_addys.contains(&nft.contract_address) {
                return Err(ContractError::NotWhitelist {
                    which: nft.contract_address.into_string(),
                });
            };
        }
    }

    Ok(())
}

pub fn is_nft_whitelisted(nft_addr: &Addr, config: &Config) -> Result<(), ContractError> {
    let wl_nfts: Vec<_> = config.whitelist_nft.iter().map(|double| double.1.clone()).collect();

    if !wl_nfts.contains(nft_addr) {
        return Err(ContractError::NotWhitelist {
            which: nft_addr.to_string(),
        });
    };

    Ok(())
}

#[cw_serde]
pub struct EzTimeStruct {
    pub year: u32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

pub trait EzTime {
    fn eztime_struct(&self) -> StdResult<EzTimeStruct>;
    fn eztime_string(&self) -> StdResult<String>;
}

impl EzTime for cosmwasm_std::Timestamp {
    fn eztime_struct(&self) -> StdResult<EzTimeStruct> {
        let seconds = &self.seconds();
        let nano = &self.subsec_nanos();

        let Some(dt) = NaiveDateTime::from_timestamp_opt(*seconds as i64, *nano as u32) else {
            return Err(StdError::GenericErr { msg: "Invalid Timestamp".to_string() });
        };

        Ok(EzTimeStruct {
            year: dt.year() as u32,
            month: dt.month(),
            day: dt.day(),
            hour: dt.hour(),
            minute: dt.minute(),
            second: dt.second(),
        })
    }

    fn eztime_string(&self) -> StdResult<String> {
        let seconds = &self.seconds();
        let nano = &self.subsec_nanos();

        let Some(dt) = NaiveDateTime::from_timestamp_opt(*seconds as i64, *nano as u32) else {
            return Err(StdError::GenericErr { msg: "Invalid Timestamp".to_string() });
        };

        match dt.month() {
            1 => Ok(format!(
                "January {}, {} | {}:{}:{} UTC",
                dt.day(),
                dt.year(),
                dt.hour(),
                dt.minute(),
                dt.second()
            )),
            2 => Ok(format!(
                "February {}, {} | {}:{}:{} UTC",
                dt.day(),
                dt.year(),
                dt.hour(),
                dt.minute(),
                dt.second()
            )),
            3 => Ok(format!(
                "March {}, {} | {}:{}:{} UTC",
                dt.day(),
                dt.year(),
                dt.hour(),
                dt.minute(),
                dt.second()
            )),
            4 => Ok(format!(
                "April {}, {} | {}:{}:{} UTC",
                dt.day(),
                dt.year(),
                dt.hour(),
                dt.minute(),
                dt.second()
            )),
            5 => Ok(format!(
                "May {}, {} | {}:{}:{} UTC",
                dt.day(),
                dt.year(),
                dt.hour(),
                dt.minute(),
                dt.second()
            )),
            6 => Ok(format!(
                "June {}, {} | {}:{}:{} UTC",
                dt.day(),
                dt.year(),
                dt.hour(),
                dt.minute(),
                dt.second()
            )),
            7 => Ok(format!(
                "July {}, {} | {}:{}:{} UTC",
                dt.day(),
                dt.year(),
                dt.hour(),
                dt.minute(),
                dt.second()
            )),
            8 => Ok(format!(
                "August {}, {} | {}:{}:{} UTC",
                dt.day(),
                dt.year(),
                dt.hour(),
                dt.minute(),
                dt.second()
            )),
            9 => Ok(format!(
                "September {}, {} | {}:{}:{} UTC",
                dt.day(),
                dt.year(),
                dt.hour(),
                dt.minute(),
                dt.second()
            )),
            10 => Ok(format!(
                "October {}, {} | {}:{}:{} UTC",
                dt.day(),
                dt.year(),
                dt.hour(),
                dt.minute(),
                dt.second()
            )),
            11 => Ok(format!(
                "November {}, {} | {}:{}:{} UTC",
                dt.day(),
                dt.year(),
                dt.hour(),
                dt.minute(),
                dt.second()
            )),
            12 => Ok(format!(
                "December {}, {} | {}:{}:{} UTC",
                dt.day(),
                dt.year(),
                dt.hour(),
                dt.minute(),
                dt.second()
            )),
            _ => Err(StdError::GenericErr {
                msg: "Invalid Timestamp".to_string(),
            }),
        }
    }
}

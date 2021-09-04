/**
 * Holo-REA satisfaction remote index zome API definition
 *
 * Manages indexes for querying `EconomicEvents` against remote `Satisfactions`.
 *
 * Defines the top-level zome configuration needed by Holochain's build system
 * to bundle the app. This basically involves wiring up the helper methods from the
 * related `_lib` module into a packaged zome WASM binary.
 *
 * @package Holo-REA
 */
use hdk::prelude::*;

use hc_zome_rea_satisfaction_lib_destination::*;
use hc_zome_rea_satisfaction_rpc::*;
use hc_zome_rea_satisfaction_storage_consts::*;
use hc_zome_rea_economic_event_storage_consts::EVENT_ENTRY_TYPE;

#[hdk_extern]
fn entry_defs(_: ()) -> ExternResult<EntryDefsCallbackResult> {
    Ok(EntryDefsCallbackResult::from(vec![
        Path::entry_def(),
        EntryDef {
            id: SATISFACTION_ENTRY_TYPE.into(),
            visibility: EntryVisibility::Public,
            crdt_type: CrdtType,
            required_validations: 1.into(),
            required_validation_type: RequiredValidationType::default(),
        }
    ]))
}

#[hdk_extern]
fn satisfaction_created(CreateParams { satisfaction }: CreateParams) -> ExternResult<ResponseData> {
    Ok(receive_create_satisfaction(SATISFACTION_ENTRY_TYPE, EVENT_ENTRY_TYPE, satisfaction)?)
}

#[hdk_extern]
fn get_satisfaction(ByAddress { address }: ByAddress<SatisfactionAddress>) -> ExternResult<ResponseData> {
    Ok(receive_get_satisfaction(SATISFACTION_ENTRY_TYPE, address)?)
}

#[hdk_extern]
fn satisfaction_updated(UpdateParams { satisfaction }: UpdateParams) -> ExternResult<ResponseData> {
    Ok(receive_update_satisfaction(SATISFACTION_ENTRY_TYPE, EVENT_ENTRY_TYPE, satisfaction)?)
}

#[hdk_extern]
fn satisfaction_deleted(ByHeader { address }: ByHeader) -> ExternResult<bool> {
    Ok(receive_delete_satisfaction(address)?)
}
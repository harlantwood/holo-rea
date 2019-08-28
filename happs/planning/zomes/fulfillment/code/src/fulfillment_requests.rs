/**
 * Handling for `Fulfillment` related behaviours as they relate to `Commitment`s
 */

use hdk::{
    PUBLIC_TOKEN,
    holochain_persistence_api::{
        cas::content::Address,
    },
    error::ZomeApiResult,
    error::ZomeApiError,
    call,
};
use hdk_graph_helpers::{
    records::{
        create_record,
        read_record_entry,
        update_record,
        delete_record,
    },
    links::{
        link_entries_bidir,
        get_links_and_load_entry_data,
    },
};

use vf_planning::identifiers::{
    BRIDGED_OBSERVATION_DHT,
};
use vf_planning::identifiers::{
    FULFILLMENT_ENTRY_TYPE,
    FULFILLMENT_BASE_ENTRY_TYPE,
    FULFILLMENT_FULFILLS_LINK_TYPE,
    FULFILLMENT_FULFILLS_LINK_TAG,
    COMMITMENT_FULFILLEDBY_LINK_TYPE,
    COMMITMENT_FULFILLEDBY_LINK_TAG,
};

use vf_planning::fulfillment::{
    Entry,
    CreateRequest,
    UpdateRequest,
    ResponseData as Response,
    construct_response,
};

pub fn handle_create_fulfillment(fulfillment: CreateRequest) -> ZomeApiResult<Response> {
    hdk::debug(format!("CREATING PLN FULFILLMENT {:?}", fulfillment))?;

    let (fulfillment_address, entry_resp): (Address, Entry) = create_record(FULFILLMENT_BASE_ENTRY_TYPE, FULFILLMENT_ENTRY_TYPE, fulfillment.clone())?;

    // link entries in the local DNA
    let _results = link_entries_bidir(
        &fulfillment_address,
        fulfillment.get_fulfills().as_ref(),
        FULFILLMENT_FULFILLS_LINK_TYPE, FULFILLMENT_FULFILLS_LINK_TAG,
        COMMITMENT_FULFILLEDBY_LINK_TYPE, COMMITMENT_FULFILLEDBY_LINK_TAG,
    );

    // register in the associated foreign DNA as well
    let _pingback = call(
        BRIDGED_OBSERVATION_DHT,
        "fulfillment",
        Address::from(PUBLIC_TOKEN.to_string()),
        "fulfillment_created",
        fulfillment.into(),
    );

    Ok(construct_response(&fulfillment_address, entry_resp))
}

/// Read an individual fulfillment's details
pub fn handle_get_fulfillment(base_address: Address) -> ZomeApiResult<Response> {
    let entry = read_record_entry(&base_address)?;
    Ok(construct_response(&base_address, entry))
}

pub fn handle_update_fulfillment(fulfillment: UpdateRequest) -> ZomeApiResult<Response> {
    let base_address = fulfillment.get_id();
    let new_entry = update_record(FULFILLMENT_ENTRY_TYPE, &base_address, &fulfillment)?;
    Ok(construct_response(base_address, new_entry))
}

pub fn handle_delete_fulfillment(address: Address) -> ZomeApiResult<bool> {
    delete_record::<Entry>(&address)
}

pub fn handle_query_fulfillments(commitment: Address) -> ZomeApiResult<Vec<Response>> {
    let entries_result: ZomeApiResult<Vec<(Address, Option<Entry>)>> = get_links_and_load_entry_data(
        &commitment,
        COMMITMENT_FULFILLEDBY_LINK_TYPE,
        COMMITMENT_FULFILLEDBY_LINK_TAG
    );

    match entries_result {
        Ok(entries) => Ok(
            entries.iter()
                .map(|(entry_base_address, maybe_entry)| {
                    // :TODO: avoid cloning entry
                    match maybe_entry {
                        Some(entry) => Ok(construct_response(entry_base_address, entry.clone().into())),
                        None => Err(ZomeApiError::Internal("referenced entry not found".to_string()))
                    }
                })
                .filter_map(Result::ok)
                .collect()
        ),
        _ => Err(ZomeApiError::Internal("could not load linked addresses".to_string()))
    }
}

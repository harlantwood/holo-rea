/**
 * Holo-REA 'economic event' zome library API
 *
 * Contains helper methods that can be used to manipulate economic event data
 * structures in either the local Holochain zome, or a separate DNA-local zome.
 *
 * @package Holo-REA
 */
use paste::paste;
use hdk_records::{
    RecordAPIResult, OtherCellResult, MaybeUndefined,
    local_indexes::{
        query_root_index,
    },
    rpc::{
        call_local_zome_method,
    },
    records::{
        create_record,
        read_record_entry,
        read_record_entry_by_header,
        update_record,
        delete_record,
    },
};
use hdk_semantic_indexes_client_lib::*;
use hdk_relay_pagination::PageInfo;

pub use hc_zome_rea_economic_event_storage_consts::*;

use hc_zome_rea_economic_event_zome_api::*;
use hc_zome_rea_economic_event_storage::*;
use hc_zome_rea_economic_event_rpc::{
    CreateRequest as EconomicEventCreateRequest,
    UpdateRequest as EconomicEventUpdateRequest,
    EventResponseCollection as Collection,
    EventResponseEdge as Edge,
};
use hc_zome_rea_economic_resource_rpc::{ CreationPayload as ResourceCreationPayload };

use hc_zome_rea_economic_resource_storage::{
    EntryData as EconomicResourceData,
};
use hc_zome_rea_economic_resource_lib::{
    construct_response_record as construct_resource_response,
    get_link_fields as get_resource_link_fields,
};


/// Properties accessor for zome config.
fn read_economic_resource_index_zome(conf: DnaConfigSlice) -> Option<String> {
    conf.economic_event.economic_resource_index_zome
}

/// Trait object defining the default ValueFlows EconomicResource zome API.
/// 'Permissable' denotes the interface as a highly-permissable one, where little
/// validation on entry contents is performed.
pub struct EconomicEventZomePermissableDefault;

impl API for EconomicEventZomePermissableDefault {
    type S = &'static str;

    fn create_economic_event(
        entry_def_id: Self::S, process_entry_def_id: Self::S,
        event: EconomicEventCreateRequest, new_inventoried_resource: Option<ResourceCreateRequest>
    ) -> RecordAPIResult<ResponseData> {
        let mut resources_affected: Vec<(RevisionHash, EconomicResourceAddress, EconomicResourceData, EconomicResourceData)> = vec![];
        let mut resource_created: Option<(RevisionHash, EconomicResourceAddress, EconomicResourceData)> = None;

        // if the event observes a new resource, create that resource & return it in the response
        if let Some(economic_resource) = new_inventoried_resource {
            let new_resource = handle_create_inventory_from_event(
                &economic_resource, &event,
            )?;
            resource_created = Some(new_resource.clone());
            resources_affected.push((new_resource.0, new_resource.1, new_resource.2.clone(), new_resource.2));
        }

        // update any linked resources affected by the event
        resources_affected.append(&mut handle_update_resource_inventory(&event)?);

        // Now that the resource updates have succeeded, write the event.
        // Note we ignore the revision ID because events can't be edited (only underwritten by subsequent events)
        // :TODO: rethinking this, it's probably the event that should be written first, and the resource
        // validation should eventually depend on an event already having been authored.
        let (revision_id, event_address, event_entry) = handle_create_economic_event_record(
            &entry_def_id,
            &event, match &resource_created {
                Some(data) => Some(data.1.to_owned()),
                None => None,
            },
        )?;

        // Link any affected resources to this event so that we can pull all the events which affect any resource
        for resource_data in resources_affected.iter() {
            create_index!(Local(economic_event.affects(&(resource_data.1)), economic_resource.affected_by(&event_address)))?;
        }

        match resource_created {
            Some((resource_revision_id, resource_addr, resource_entry)) => {
                construct_response_with_resource(
                    &event_address, &revision_id, &event_entry, get_link_fields(&event_address)?,
                    Some(resource_addr.clone()), &resource_revision_id, resource_entry, get_resource_link_fields(
                        &entry_def_id, &process_entry_def_id, &resource_addr
                    )?
                )
            },
            None => {
                // :TODO: pass results from link creation rather than re-reading
                construct_response(&event_address, &revision_id, &event_entry, get_link_fields(&event_address)?)
            },
        }
    }

    fn get_economic_event(entry_def_id: Self::S, address: EconomicEventAddress) -> RecordAPIResult<ResponseData> {
        let (revision, base_address, entry) = read_record_entry::<EntryData, EntryStorage, _,_>(&entry_def_id, address.as_ref())?;
        construct_response(&base_address, &revision, &entry, get_link_fields(&address)?)
    }

    fn update_economic_event(entry_def_id: Self::S, event: EconomicEventUpdateRequest) -> RecordAPIResult<ResponseData> {
        let address = event.get_revision_id().to_owned();
        let (revision_id, identity_address, new_entry, _prev_entry): (_, EconomicEventAddress, EntryData, EntryData) = update_record(&entry_def_id, &address, event)?;

        // :TODO: optimise this- should pass results from `replace_direct_index` instead of retrieving from `get_link_fields` where updates
        construct_response(&identity_address, &revision_id, &new_entry, get_link_fields(&identity_address)?)
    }

    fn delete_economic_event(revision_id: RevisionHash) -> RecordAPIResult<bool> {
        // read any referencing indexes
        let (base_address, entry) = read_record_entry_by_header::<EntryData, EntryStorage, _>(&revision_id)?;

        // handle link fields
        if let Some(process_address) = entry.input_of {
            update_index!(Local(economic_event.input_of.not(&vec![process_address.to_owned()]), process.inputs(&base_address)))?;
        }
        if let Some(process_address) = entry.output_of {
            update_index!(Local(economic_event.output_of.not(&vec![process_address.to_owned()]), process.outputs(&base_address)))?;
        }
        if let Some(agreement_address) = entry.realization_of {
            let _ = update_index!(Remote(economic_event.realization_of.not(&vec![agreement_address.to_owned()]), agreement.economic_events(&base_address)));
        }

        // :TODO: handle cleanup of foreign key fields? (fulfillment, satisfaction)
        // May not be needed due to cross-record deletion validation logic.

        // delete entry last as it must be present in order for links to be removed
        delete_record::<EntryStorage, RevisionHash>(&revision_id)
    }

    fn get_all_economic_events(entry_def_id: Self::S) -> RecordAPIResult<Collection> {
        let entries_result = query_root_index::<EntryData, EntryStorage, _,_>(&entry_def_id)?;
        handle_list_output(entries_result)
    }
}

// API logic handlers

/// Properties accessor for zome config.
fn read_economic_event_index_zome(conf: DnaConfigSlice) -> Option<String> {
    Some(conf.economic_event.index_zome)
}
/// Properties accessor for zome config.
///
/// :TODO: should this be configurable as an array, to allow shared process planning spaces to be driven by multiple event logs?
///
fn read_process_index_zome(conf: DnaConfigSlice) -> Option<String> {
    conf.economic_event.process_index_zome
}

fn handle_create_economic_event_record<S>(entry_def_id: S, event: &EconomicEventCreateRequest, resource_address: Option<EconomicResourceAddress>,
) -> RecordAPIResult<(RevisionHash, EconomicEventAddress, EntryData)>
    where S: AsRef<str>
{
    let (revision_id, base_address, entry_resp): (_, EconomicEventAddress, EntryData) = create_record(
        &entry_def_id,
        match resource_address {
            Some(addr) => event.with_inventoried_resource(&addr),
            None => event.to_owned(),
        }
    )?;

    // handle link fields
    // :TODO: propagate errors
    if let EconomicEventCreateRequest { input_of: MaybeUndefined::Some(input_of), .. } = event {
        create_index!(Local(economic_event.input_of(input_of), process.inputs(&base_address)))?;
    };
    if let EconomicEventCreateRequest { output_of: MaybeUndefined::Some(output_of), .. } = event {
        create_index!(Local(economic_event.output_of(output_of), process.outputs(&base_address)))?;
    };
    if let EconomicEventCreateRequest { realization_of: MaybeUndefined::Some(realization_of), .. } = event {
        create_index!(Remote(economic_event.realization_of(realization_of), agreement.realized(&base_address)))?;
    };

    Ok((revision_id, base_address, entry_resp))
}

/// Properties accessor for zome config.
///
/// :TODO: should this be configurable as an array, to allow multiple inventories to be driven by the same event log?
///
fn read_resource_zome(conf: DnaConfigSlice) -> Option<String> {
    conf.economic_event.economic_resource_zome
}

/// Handle creation of new resources via events + resource metadata
///
fn handle_create_inventory_from_event(
    economic_resource: &ResourceCreateRequest, event: &CreateRequest,
) -> OtherCellResult<(RevisionHash, EconomicResourceAddress, EconomicResourceData)>
{
    Ok(call_local_zome_method(
        read_resource_zome,
        INVENTORY_CREATION_API_METHOD.to_string(),
        resource_creation(&event, &economic_resource),
    )?)
}

fn resource_creation(event: &CreateRequest, resource: &ResourceCreateRequest) -> ResourceCreationPayload {
    ResourceCreationPayload {
        event: event.to_owned(),
        resource: resource.to_owned(),
    }
}

/// Handle alteration of existing resources via events
///
fn handle_update_resource_inventory(
    event: &EconomicEventCreateRequest,
) -> RecordAPIResult<Vec<(RevisionHash, EconomicResourceAddress, EconomicResourceData, EconomicResourceData)>>
{
    Ok(call_local_zome_method(
        read_resource_zome,
        INVENTORY_UPDATE_API_METHOD.to_string(),
        event,
    )?)
}

fn handle_list_output(entries_result: Vec<RecordAPIResult<(RevisionHash, EconomicEventAddress, EntryData)>>) -> RecordAPIResult<Collection> {
    let edges = entries_result.iter()
        .cloned()
        .filter_map(Result::ok)
        .map(|(revision_id, entry_base_address, entry)| {
            construct_list_response(
                &entry_base_address, &revision_id, &entry,
                get_link_fields(&entry_base_address)?,
            )
        })
        .filter_map(Result::ok); // :TODO: handle internal errors in record construction (eg. corrupted DHT links)

    let mut edge_cursors = edges.clone().map(|e| { e.cursor });
    let first_cursor = edge_cursors.next().unwrap_or("0".to_string());

    Ok(Collection {
        edges: edges.collect(),
        page_info: PageInfo {
            end_cursor: edge_cursors.last().unwrap_or(first_cursor.clone()),
            start_cursor: first_cursor,
            // :TODO:
            has_next_page: true,
            has_previous_page: true,
            page_limit: None,
            total_count: None,
        },
    })
}

/**
 * Create response from input DHT primitives
 *
 * :TODO: determine if possible to construct `Response` with refs to fields of `e`, rather than cloning memory
 */
pub fn construct_response_with_resource<'a>(
    event_address: &EconomicEventAddress,
    revision_id: &RevisionHash,
    event: &EntryData, (
        fulfillments,
        satisfactions,
    ): (
        Vec<FulfillmentAddress>,
        Vec<SatisfactionAddress>,
    ),
    resource_address: Option<EconomicResourceAddress>,
    resource_revision_id: &RevisionHash,
    resource: EconomicResourceData, (
        contained_in,
        stage,
        state,
        contains,
     ): (
        Option<EconomicResourceAddress>,
        Option<ProcessSpecificationAddress>,
        Option<ActionId>,
        Vec<EconomicResourceAddress>,
    ),
) -> RecordAPIResult<ResponseData> {
    Ok(ResponseData {
        economic_event: Response {
            id: event_address.to_owned(),
            revision_id: revision_id.to_owned(),
            action: event.action.to_owned(),
            note: event.note.to_owned(),
            input_of: event.input_of.to_owned(),
            output_of: event.output_of.to_owned(),
            provider: event.provider.to_owned(),
            receiver: event.receiver.to_owned(),
            resource_inventoried_as: event.resource_inventoried_as.to_owned(),
            to_resource_inventoried_as: event.to_resource_inventoried_as.to_owned(),
            resource_classified_as: event.resource_classified_as.to_owned(),
            resource_conforms_to: event.resource_conforms_to.to_owned(),
            resource_quantity: event.resource_quantity.to_owned(),
            effort_quantity: event.effort_quantity.to_owned(),
            has_beginning: event.has_beginning.to_owned(),
            has_end: event.has_end.to_owned(),
            has_point_in_time: event.has_point_in_time.to_owned(),
            at_location: event.at_location.to_owned(),
            agreed_in: event.agreed_in.to_owned(),
            triggered_by: event.triggered_by.to_owned(),
            realization_of: event.realization_of.to_owned(),
            in_scope_of: event.in_scope_of.to_owned(),
            fulfills: fulfillments.to_owned(),
            satisfies: satisfactions.to_owned(),
        },
        economic_resource: match resource_address {
            Some(addr) => Some(construct_resource_response(&addr, &resource_revision_id, &resource, (contained_in, stage, state, contains))?),
            None => None,
        },
    })
}

// Same as above, but omits EconomicResource object
pub fn construct_response<'a>(
    address: &EconomicEventAddress, revision_id: &RevisionHash, e: &EntryData, (
        fulfillments,
        satisfactions,
    ): (
        Vec<FulfillmentAddress>,
        Vec<SatisfactionAddress>,
    )
) -> RecordAPIResult<ResponseData> {
    Ok(ResponseData {
        economic_event: Response {
            id: address.to_owned(),
            revision_id: revision_id.to_owned(),
            action: e.action.to_owned(),
            note: e.note.to_owned(),
            input_of: e.input_of.to_owned(),
            output_of: e.output_of.to_owned(),
            provider: e.provider.to_owned(),
            receiver: e.receiver.to_owned(),
            resource_inventoried_as: e.resource_inventoried_as.to_owned(),
            to_resource_inventoried_as: e.to_resource_inventoried_as.to_owned(),
            resource_classified_as: e.resource_classified_as.to_owned(),
            resource_conforms_to: e.resource_conforms_to.to_owned(),
            resource_quantity: e.resource_quantity.to_owned(),
            effort_quantity: e.effort_quantity.to_owned(),
            has_beginning: e.has_beginning.to_owned(),
            has_end: e.has_end.to_owned(),
            has_point_in_time: e.has_point_in_time.to_owned(),
            at_location: e.at_location.to_owned(),
            agreed_in: e.agreed_in.to_owned(),
            triggered_by: e.triggered_by.to_owned(),
            realization_of: e.realization_of.to_owned(),
            in_scope_of: e.in_scope_of.to_owned(),
            fulfills: fulfillments.to_owned(),
            satisfies: satisfactions.to_owned(),
        },
        economic_resource: None,
    })
}

pub fn construct_list_response<'a>(
    address: &EconomicEventAddress, revision_id: &RevisionHash, e: &EntryData, (
        fulfillments,
        satisfactions,
    ): (
        Vec<FulfillmentAddress>,
        Vec<SatisfactionAddress>,
    )
) -> RecordAPIResult<Edge> {
    let record_cursor: Vec<u8> = address.to_owned().into();
    Ok(Edge {
        node: construct_response(address, revision_id, e, (fulfillments, satisfactions))?.economic_event,
        // :TODO: use HoloHashb64 once API stabilises
        cursor: String::from_utf8(record_cursor).unwrap_or("".to_string())
    })
}

// @see construct_response
pub fn get_link_fields(event: &EconomicEventAddress) -> RecordAPIResult<(
    Vec<FulfillmentAddress>,
    Vec<SatisfactionAddress>,
)> {
    Ok((
        read_index!(economic_event(event).fulfills)?,
        read_index!(economic_event(event).satisfies)?,
    ))
}

// #[cfg(test)]
// mod tests {
//     use super::*;

    // #[test]
    // fn test_derived_fields() {
    //     let e = Entry { note: Some("a note".into()), ..Entry::default() };
    //     assert_eq!(e.note, Some("a note".into()))
    // }

    // :TODO: unit tests for type conversions... though maybe these should be macro tests, not tests for every single record type
// }

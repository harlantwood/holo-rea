// trace_macros!(true);

use hdk::holochain_core_types::{
    cas::content::Address,
};

use super::super::knowledge::action::Action;

use super::super::core::{
    measurement::QuantityValue,
};

use super::super::core::type_aliases::{
    Timestamp,
    ExternalURL,
    LocationAddress,
    AgentAddress,
    ResourceAddress,
    ProcessOrTransferAddress,
    ResourceSpecificationAddress,
};

vfRecord! {
    struct Commitment {
        action: Action,
        input_of: ProcessOrTransferAddress,
        output_of: ProcessOrTransferAddress,
        provider: AgentAddress,
        receiver: AgentAddress,
        resource_inventoried_as: ResourceAddress,
        resource_classified_as: Option<Vec<ExternalURL>>,
        resource_conforms_to: ResourceSpecificationAddress,
        quantified_as: Option<QuantityValue>,
        has_beginning: Timestamp,
        has_end: Timestamp,
        has_point_in_time: Timestamp,
        before: Timestamp,
        after: Timestamp,
        at_location: LocationAddress,
        plan: PlanAddress,
        finished: bool,
        in_scope_of: Option<Vec<String>>,
    }
}

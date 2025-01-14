/**
 * Storage constants for zome entry & link type identifiers
 *
 * Used by modules interfacing with the underlying Holochain storage system directly.
 *
 * @package Holo-REA
 */
pub const AGREEMENT_ENTRY_TYPE: &str = "vf_agreement";

pub const AGREEMENT_EVENTS_LINK_TAG: &str = "economic_events";
pub const AGREEMENT_COMMITMENTS_LINK_TAG: &str = "commitments";

pub const AGREEMENT_COMMITMENTS_READ_API_METHOD: &str = "_internal_read_agreement_clauses";
pub const AGREEMENT_EVENTS_READ_API_METHOD: &str = "_internal_read_agreement_realizations";

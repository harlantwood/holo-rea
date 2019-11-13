use hdk::{
    holochain_json_api::{ json::JsonString, error::JsonError },
};
use holochain_json_derive::{ DefaultJson };

use hdk_graph_helpers::{
    MaybeUndefined,
    record_interface::Updateable,
};

use vf_core::type_aliases::{
    UnitAddress,
};

#[derive(Serialize, Deserialize, Debug, DefaultJson, Default, Clone)]
pub struct Entry {
    name: String,
    symbol: String,
}

/// Handles update operations by merging any newly provided fields
impl Updateable<UpdateRequest> for Entry {
    fn update_with(&self, e: &UpdateRequest) -> Entry {
        Entry {
            name:   if e.name == MaybeUndefined::Undefined   { self.name.to_owned()   } else { e.name.to_owned().to_option().unwrap() },
            symbol: if e.symbol == MaybeUndefined::Undefined { self.symbol.to_owned() } else { e.symbol.to_owned().to_option().unwrap() },
        }
    }
}

/// I/O struct to describe the complete input record, including all managed links
#[derive(Serialize, Deserialize, Debug, DefaultJson, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CreateRequest {
    name: String,
    symbol: String,
}

impl<'a> CreateRequest {
    // :TODO: accessors for field data
}

#[derive(Serialize, Deserialize, Debug, DefaultJson, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRequest {
    id: UnitAddress,
    name: MaybeUndefined<String>,
    symbol: MaybeUndefined<String>,
}

impl<'a> UpdateRequest {
    pub fn get_id(&'a self) -> &UnitAddress {
        &self.id
    }

    // :TODO: accessors for other field data
}

/// I/O struct to describe the complete output record, including all managed link fields
#[derive(Serialize, Deserialize, Debug, DefaultJson, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    id: UnitAddress,
    name: String,
    symbol: String,
}

/// I/O struct to describe what is returned outside the gateway
#[derive(Serialize, Deserialize, Debug, DefaultJson, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ResponseData {
    process: Response,
}

/// Pick relevant fields out of I/O record into underlying DHT entry
impl From<CreateRequest> for Entry {
    fn from(e: CreateRequest) -> Entry {
        Entry {
            name: e.name.into(),
            symbol: e.symbol.into(),
        }
    }
}

pub fn construct_response<'a>(
    address: &UnitAddress, e: &Entry
) -> ResponseData {
    ResponseData {
        process: Response {
            // entry fields
            id: address.to_owned(),
            name: e.name.to_owned(),
            symbol: e.symbol.to_owned(),
        }
    }
}
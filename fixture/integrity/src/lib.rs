use hdi::prelude::*;

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    TestType(TestType),
}

#[hdk_link_types]
pub enum LinkTypes {
    Link,
}

#[hdk_entry_helper]
pub struct TestType {
    pub value: String,
}

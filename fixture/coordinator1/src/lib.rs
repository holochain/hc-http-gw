use hdk::prelude::*;
use integrity::*;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateResponse {
    pub created: ActionHashB64,
}

#[hdk_extern]
pub fn create_1() -> ExternResult<CreateResponse> {
    let time = sys_time()?;
    let created = create_entry(EntryTypes::TestType(TestType {
        value: format!("create_1_{time}"),
    }))?;

    create_link(base(), created.clone(), LinkTypes::Link, ())?;

    Ok(CreateResponse { created: created.into() })
}

#[hdk_extern]
pub fn get_all_1() -> ExternResult<Vec<TestType>> {
    let links = get_links(GetLinksInputBuilder::try_new(base(), LinkTypes::Link)?.build())?;

    let mut out = Vec::new();
    for link in links {
        let Some(target) = link.target.into_any_dht_hash() else {
            continue;
        };

        let Some(record) = get(target, GetOptions::local())? else {
            continue;
        };

        let Ok(Some(e)) = record.entry.to_app_option::<TestType>() else {
            continue;
        };

        out.push(e);
    }

    Ok(out)
}

#[hdk_extern]
pub fn get_mine(agent_pub_key: AgentPubKey) -> ExternResult<Vec<TestType>> {
    let links = get_links(GetLinksInputBuilder::try_new(base(), LinkTypes::Link)?.build())?;

    let mut out = Vec::new();
    for link in links {
        if link.author != agent_pub_key {
            continue;
        }

        let Some(target) = link.target.into_any_dht_hash() else {
            continue;
        };

        let Some(record) = get(target, GetOptions::local())? else {
            continue;
        };

        let Ok(Some(e)) = record.entry.to_app_option::<TestType>() else {
            continue;
        };

        out.push(e);
    }

    Ok(out)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetWithLimitRequest {
    limit: usize,
}

#[hdk_extern]
pub fn get_limited(request: GetWithLimitRequest) -> ExternResult<Vec<TestType>> {
    let links = get_links(GetLinksInputBuilder::try_new(base(), LinkTypes::Link)?.build())?;

    let mut out = Vec::new();
    for link in links {
        if out.len() >= request.limit {
            break;
        }

        let Some(target) = link.target.into_any_dht_hash() else {
            continue;
        };

        let Some(record) = get(target, GetOptions::local())? else {
            continue;
        };

        let Ok(Some(e)) = record.entry.to_app_option::<TestType>() else {
            continue;
        };

        out.push(e);
    }

    Ok(out)
}

fn base() -> AnyLinkableHash {
    EntryHash::from_raw_36(vec![1; 36]).into()
}

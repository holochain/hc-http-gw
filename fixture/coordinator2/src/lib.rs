use hdk::prelude::*;
use integrity::*;

#[hdk_extern]
pub fn create_2() -> ExternResult<ActionHash> {
    let time = sys_time()?;
    let created = create_entry(EntryTypes::TestType(TestType {
        value: format!("create_2_{time}"),
    }))?;

    create_link(base(), created.clone(), LinkTypes::Link, ())?;

    Ok(created)
}

#[hdk_extern]
pub fn get_all_2() -> ExternResult<Vec<TestType>> {
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

fn base() -> AnyLinkableHash {
    EntryHash::from_raw_36(vec![2; 36]).into()
}

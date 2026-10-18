use std::collections::BTreeMap;

use super::config_store::ConfigStore;
use super::error::AppResult;
use super::model::{Credential, Site};
use super::model_rules;

#[derive(Debug, Clone, Default)]
pub struct CredentialImportEntry {
    pub name: String,
    pub host: String,
    pub credential_type: String,
    pub token: String,
    pub user_id: i32,
}

#[derive(Debug, Clone, Default)]
pub struct CredentialImportResult {
    pub imported: i32,
    pub added_sites: i32,
    pub errors: Vec<String>,
}

pub fn apply(
    store: &ConfigStore,
    entries: &[CredentialImportEntry],
) -> AppResult<CredentialImportResult> {
    let mut errors: Vec<String> = Vec::new();
    let mut drafts: Vec<(Site, Credential, usize)> = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        let row = index + 1;
        let draft = Site {
            host: entry.host.clone(),
            name: entry.name.clone(),
            ..Site::default()
        };
        let Some(site) = model_rules::normalize_site(Some(&draft)) else {
            errors.push(format!("第 {row} 条：站点地址不合法"));
            continue;
        };
        let kind = if entry.credential_type.trim().is_empty() {
            "token".to_string()
        } else {
            entry.credential_type.trim().to_string()
        };
        if !kind.eq_ignore_ascii_case("token") {
            errors.push(format!("第 {row} 条：凭据类型不受支持"));
            continue;
        }
        let credential = model_rules::normalize_credential(Some(&Credential {
            credential_type: "token".to_string(),
            token: entry.token.clone(),
            user_id: entry.user_id,
            ..Credential::default()
        }));
        if !model_rules::is_configured(Some(&credential)) {
            errors.push(format!("第 {row} 条：访问令牌为空"));
            continue;
        }
        drafts.push((site, credential, row));
    }

    let mut patches: BTreeMap<String, Vec<Credential>> = BTreeMap::new();
    let mut added_sites = 0;
    for (draft_site, credential, row) in drafts {
        let site = match store.find_by_host(&draft_site.host) {
            Some(existing) => Some(existing),
            None => match store.add_site(&draft_site.host, &draft_site.name) {
                Ok(created) => {
                    added_sites += 1;
                    Some(created)
                }
                Err(error) => {
                    errors.push(format!("第 {row} 条：{}", error.message()));
                    None
                }
            },
        };
        let Some(site) = site else {
            continue;
        };
        patches.entry(site.id.clone()).or_default().push(credential);
    }

    let mut imported = 0;
    store.batch(|store| {
        for (site_id, groups) in &patches {
            imported += store.add_credential_groups(site_id, groups)?;
        }
        Ok(())
    })?;

    Ok(CredentialImportResult {
        imported,
        added_sites,
        errors,
    })
}

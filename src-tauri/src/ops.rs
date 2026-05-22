use crate::db::get_db_path;
use crate::models::{
    AccessEndpoint, AssetFolder, AssetSessionConnectResult, AssetTag, AssetUpsertPayload,
    AuditEvent, Connection as SshConnection, CredentialRef, Environment, HostAsset, JobRun,
    JobTemplate, SavedAssetView, SyncState,
};
use crate::ssh::{client, client::AppState, command};
use rusqlite::{params, Connection as SqliteConnection};
use tauri::{AppHandle, State};

fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn slugify(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn parse_labels(raw: Option<String>) -> Vec<String> {
    raw.unwrap_or_default()
        .split(',')
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

fn join_labels(labels: &[String]) -> String {
    labels
        .iter()
        .map(|label| label.trim())
        .filter(|label| !label.is_empty())
        .collect::<Vec<_>>()
        .join(",")
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value.and_then(|item| {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn default_credential_name(asset_name: &str, credential_kind: &str) -> String {
    match credential_kind {
        "sshKey" => format!("{} key credential", asset_name),
        "token" => format!("{} token credential", asset_name),
        _ => format!("{} password credential", asset_name),
    }
}

pub fn init_ops_schema(app_handle: &AppHandle) -> rusqlite::Result<()> {
    let db_path = get_db_path(app_handle);
    let conn = SqliteConnection::open(db_path)?;

    conn.execute_batch("PRAGMA foreign_keys = OFF;")?;

    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS asset_folders (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            parent_id INTEGER,
            color TEXT,
            FOREIGN KEY(parent_id) REFERENCES asset_folders(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS environments (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            slug TEXT NOT NULL UNIQUE,
            color TEXT,
            description TEXT
        );

        CREATE TABLE IF NOT EXISTS asset_tags (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            color TEXT
        );

        CREATE TABLE IF NOT EXISTS host_assets (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            host TEXT NOT NULL,
            port INTEGER NOT NULL DEFAULT 22,
            username TEXT NOT NULL,
            password TEXT,
            auth_type TEXT NOT NULL DEFAULT 'password',
            ssh_key_id INTEGER,
            jump_host TEXT,
            jump_port INTEGER,
            jump_username TEXT,
            jump_password TEXT,
            platform TEXT NOT NULL DEFAULT 'Linux',
            folder_id INTEGER,
            env_id INTEGER,
            labels_csv TEXT NOT NULL DEFAULT '',
            owner TEXT,
            criticality TEXT NOT NULL DEFAULT 'medium',
            default_workspace_path TEXT,
            access_endpoint_id INTEGER,
            bastion_chain_id TEXT,
            health_summary TEXT,
            last_accessed_at INTEGER,
            is_favorite INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL DEFAULT (strftime('%s','now')),
            updated_at INTEGER NOT NULL DEFAULT (strftime('%s','now')),
            FOREIGN KEY(folder_id) REFERENCES asset_folders(id) ON DELETE SET NULL,
            FOREIGN KEY(env_id) REFERENCES environments(id) ON DELETE SET NULL,
            FOREIGN KEY(ssh_key_id) REFERENCES ssh_keys(id) ON DELETE SET NULL
        );

        CREATE TABLE IF NOT EXISTS host_asset_tags (
            asset_id INTEGER NOT NULL,
            tag_id INTEGER NOT NULL,
            PRIMARY KEY(asset_id, tag_id),
            FOREIGN KEY(asset_id) REFERENCES host_assets(id) ON DELETE CASCADE,
            FOREIGN KEY(tag_id) REFERENCES asset_tags(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS credential_refs (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            credential_kind TEXT NOT NULL,
            username TEXT,
            secret TEXT,
            ssh_key_id INTEGER,
            asset_id INTEGER,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            FOREIGN KEY(ssh_key_id) REFERENCES ssh_keys(id) ON DELETE SET NULL,
            FOREIGN KEY(asset_id) REFERENCES host_assets(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS access_endpoints (
            id INTEGER PRIMARY KEY,
            asset_id INTEGER NOT NULL,
            name TEXT NOT NULL,
            host TEXT NOT NULL,
            port INTEGER NOT NULL DEFAULT 22,
            username TEXT NOT NULL,
            auth_type TEXT,
            credential_ref_id INTEGER,
            ssh_key_id INTEGER,
            jump_host TEXT,
            jump_port INTEGER,
            jump_username TEXT,
            jump_password TEXT,
            FOREIGN KEY(asset_id) REFERENCES host_assets(id) ON DELETE CASCADE,
            FOREIGN KEY(credential_ref_id) REFERENCES credential_refs(id) ON DELETE SET NULL,
            FOREIGN KEY(ssh_key_id) REFERENCES ssh_keys(id) ON DELETE SET NULL
        );

        CREATE TABLE IF NOT EXISTS saved_views (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            query_json TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS job_templates (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            command TEXT NOT NULL,
            scope_type TEXT NOT NULL DEFAULT 'asset',
            scope_value TEXT,
            risk_level TEXT NOT NULL DEFAULT 'medium',
            requires_confirmation INTEGER NOT NULL DEFAULT 1,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS job_runs (
            id INTEGER PRIMARY KEY,
            asset_id INTEGER,
            session_id TEXT,
            template_id INTEGER,
            command TEXT NOT NULL,
            status TEXT NOT NULL,
            output TEXT,
            risk_level TEXT NOT NULL DEFAULT 'medium',
            initiated_by TEXT,
            source TEXT,
            created_at INTEGER NOT NULL,
            completed_at INTEGER,
            FOREIGN KEY(asset_id) REFERENCES host_assets(id) ON DELETE SET NULL,
            FOREIGN KEY(template_id) REFERENCES job_templates(id) ON DELETE SET NULL
        );

        CREATE TABLE IF NOT EXISTS audit_events (
            id INTEGER PRIMARY KEY,
            event_type TEXT NOT NULL,
            asset_id INTEGER,
            session_id TEXT,
            job_run_id INTEGER,
            title TEXT NOT NULL,
            detail TEXT,
            severity TEXT NOT NULL DEFAULT 'info',
            metadata_json TEXT,
            created_at INTEGER NOT NULL,
            FOREIGN KEY(asset_id) REFERENCES host_assets(id) ON DELETE SET NULL,
            FOREIGN KEY(job_run_id) REFERENCES job_runs(id) ON DELETE SET NULL
        );

        CREATE TABLE IF NOT EXISTS sync_state (
            id INTEGER PRIMARY KEY,
            state_key TEXT NOT NULL UNIQUE,
            status TEXT NOT NULL,
            version INTEGER NOT NULL DEFAULT 1,
            endpoint_url TEXT,
            last_synced_at INTEGER,
            last_error TEXT,
            metadata_json TEXT,
            updated_at INTEGER NOT NULL
        );
        "#,
    )?;

    conn.execute(
        "INSERT OR IGNORE INTO asset_folders (id, name, parent_id) SELECT id, name, parent_id FROM connection_groups",
        [],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO host_assets (
            id, name, host, port, username, password, auth_type, ssh_key_id, jump_host, jump_port, jump_username, jump_password,
            platform, folder_id, owner, criticality, default_workspace_path, last_accessed_at, is_favorite, created_at, updated_at
        )
        SELECT
            id, name, host, port, username, password, COALESCE(auth_type, 'password'), ssh_key_id, jump_host, jump_port, jump_username, jump_password,
            COALESCE(os_type, 'Linux'), group_id, username, 'medium',
            CASE
                WHEN COALESCE(os_type, 'Linux') = 'Windows' THEN 'C:/Users/' || username
                ELSE '/home/' || username
            END,
            NULL, 0, strftime('%s','now'), strftime('%s','now')
        FROM connections",
        [],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO credential_refs (id, name, credential_kind, username, secret, ssh_key_id, asset_id, created_at, updated_at)
         SELECT
            1000000 + id,
            name || ' password',
            'password',
            username,
            password,
            NULL,
            id,
            strftime('%s','now'),
            strftime('%s','now')
         FROM connections
         WHERE password IS NOT NULL AND TRIM(password) <> ''",
        [],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO credential_refs (id, name, credential_kind, username, secret, ssh_key_id, asset_id, created_at, updated_at)
         SELECT
            2000000 + id,
            name,
            'sshKey',
            NULL,
            NULL,
            id,
            NULL,
            created_at,
            created_at
         FROM ssh_keys",
        [],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO access_endpoints (
            id, asset_id, name, host, port, username, auth_type, ssh_key_id, jump_host, jump_port, jump_username, jump_password
        )
        SELECT
            id, id, name || ' endpoint', host, port, username, COALESCE(auth_type, 'password'), ssh_key_id, jump_host, jump_port, jump_username, jump_password
        FROM connections",
        [],
    )?;
    conn.execute(
        "UPDATE access_endpoints
         SET credential_ref_id = CASE
            WHEN auth_type = 'key' AND ssh_key_id IS NOT NULL THEN 2000000 + ssh_key_id
            WHEN auth_type = 'password' THEN 1000000 + asset_id
            ELSE credential_ref_id
         END
         WHERE credential_ref_id IS NULL",
        [],
    )?;
    conn.execute(
        "UPDATE host_assets
         SET access_endpoint_id = COALESCE(access_endpoint_id, (
            SELECT ae.id FROM access_endpoints ae WHERE ae.asset_id = host_assets.id LIMIT 1
         )),
         updated_at = strftime('%s','now')",
        [],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO sync_state (id, state_key, status, version, updated_at)
         VALUES (1, 'local-default', 'idle', 1, strftime('%s','now'))",
        [],
    )?;

    // Clean up orphaned foreign keys before re-enabling constraints
    conn.execute(
        "UPDATE host_assets SET ssh_key_id = NULL WHERE ssh_key_id IS NOT NULL AND ssh_key_id NOT IN (SELECT id FROM ssh_keys)",
        [],
    )?;
    conn.execute(
        "UPDATE access_endpoints SET ssh_key_id = NULL WHERE ssh_key_id IS NOT NULL AND ssh_key_id NOT IN (SELECT id FROM ssh_keys)",
        [],
    )?;
    conn.execute(
        "UPDATE access_endpoints SET credential_ref_id = NULL WHERE credential_ref_id IS NOT NULL AND credential_ref_id NOT IN (SELECT id FROM credential_refs)",
        [],
    )?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;

    Ok(())
}

fn append_audit_event_with_conn(
    conn: &SqliteConnection,
    event_type: &str,
    asset_id: Option<i64>,
    session_id: Option<&str>,
    job_run_id: Option<i64>,
    title: &str,
    detail: Option<&str>,
    severity: &str,
    metadata_json: Option<&str>,
) -> Result<i64, String> {
    conn.execute(
        "INSERT INTO audit_events (event_type, asset_id, session_id, job_run_id, title, detail, severity, metadata_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            event_type,
            asset_id,
            session_id,
            job_run_id,
            title,
            detail,
            severity,
            metadata_json,
            now_ts()
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(conn.last_insert_rowid())
}

pub fn append_audit_event(
    app_handle: &AppHandle,
    event_type: &str,
    asset_id: Option<i64>,
    session_id: Option<&str>,
    job_run_id: Option<i64>,
    title: &str,
    detail: Option<&str>,
    severity: &str,
    metadata_json: Option<&str>,
) -> Result<i64, String> {
    let db_path = get_db_path(app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    append_audit_event_with_conn(
        &conn,
        event_type,
        asset_id,
        session_id,
        job_run_id,
        title,
        detail,
        severity,
        metadata_json,
    )
}

fn upsert_asset_tags(conn: &SqliteConnection, asset_id: i64, labels: &[String]) -> Result<(), String> {
    conn.execute(
        "DELETE FROM host_asset_tags WHERE asset_id = ?1",
        params![asset_id],
    )
    .map_err(|e| e.to_string())?;

    for label in labels.iter().map(|item| item.trim()).filter(|item| !item.is_empty()) {
        conn.execute(
            "INSERT OR IGNORE INTO asset_tags (name) VALUES (?1)",
            params![label],
        )
        .map_err(|e| e.to_string())?;
        let tag_id: i64 = conn
            .query_row(
                "SELECT id FROM asset_tags WHERE name = ?1",
                params![label],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR IGNORE INTO host_asset_tags (asset_id, tag_id) VALUES (?1, ?2)",
            params![asset_id, tag_id],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn map_host_asset_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<HostAsset> {
    Ok(HostAsset {
        id: row.get(0)?,
        name: row.get(1)?,
        host: row.get(2)?,
        port: row.get(3)?,
        platform: row.get(4)?,
        folder_id: row.get(5)?,
        env_id: row.get(6)?,
        labels: parse_labels(row.get(7)?),
        owner: row.get(8)?,
        criticality: row.get(9)?,
        default_workspace_path: row.get(10)?,
        access_endpoint_id: row.get(11)?,
        bastion_chain_id: row.get(12)?,
        health_summary: row.get(13)?,
        last_accessed_at: row.get(14)?,
        is_favorite: row
            .get::<_, Option<i64>>(15)?
            .map(|value| value != 0),
        group_id: row.get(16)?,
    })
}

fn map_folder_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AssetFolder> {
    Ok(AssetFolder {
        id: row.get(0)?,
        name: row.get(1)?,
        parent_id: row.get(2)?,
        color: row.get(3)?,
    })
}

fn map_environment_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Environment> {
    Ok(Environment {
        id: row.get(0)?,
        name: row.get(1)?,
        slug: row.get(2)?,
        color: row.get(3)?,
        description: row.get(4)?,
    })
}

fn map_tag_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AssetTag> {
    Ok(AssetTag {
        id: row.get(0)?,
        name: row.get(1)?,
        color: row.get(2)?,
    })
}

fn map_saved_view_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SavedAssetView> {
    Ok(SavedAssetView {
        id: row.get(0)?,
        name: row.get(1)?,
        query_json: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
    })
}

fn map_access_endpoint_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AccessEndpoint> {
    Ok(AccessEndpoint {
        id: row.get(0)?,
        asset_id: row.get(1)?,
        name: row.get(2)?,
        host: row.get(3)?,
        port: row.get(4)?,
        username: row.get(5)?,
        auth_type: row.get(6)?,
        credential_ref_id: row.get(7)?,
        ssh_key_id: row.get(8)?,
        jump_host: row.get(9)?,
        jump_port: row.get(10)?,
        jump_username: row.get(11)?,
        jump_password: row.get(12)?,
    })
}

fn map_credential_ref_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<CredentialRef> {
    Ok(CredentialRef {
        id: row.get(0)?,
        name: row.get(1)?,
        credential_kind: row.get(2)?,
        username: row.get(3)?,
        secret: row.get(4)?,
        ssh_key_id: row.get(5)?,
        asset_id: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

fn map_job_template_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<JobTemplate> {
    Ok(JobTemplate {
        id: row.get(0)?,
        name: row.get(1)?,
        command: row.get(2)?,
        scope_type: row.get(3)?,
        scope_value: row.get(4)?,
        risk_level: row.get(5)?,
        requires_confirmation: row.get::<_, i64>(6)? != 0,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

fn map_job_run_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<JobRun> {
    Ok(JobRun {
        id: row.get(0)?,
        asset_id: row.get(1)?,
        session_id: row.get(2)?,
        template_id: row.get(3)?,
        command: row.get(4)?,
        status: row.get(5)?,
        output: row.get(6)?,
        risk_level: row.get(7)?,
        initiated_by: row.get(8)?,
        source: row.get(9)?,
        created_at: row.get(10)?,
        completed_at: row.get(11)?,
    })
}

fn map_audit_event_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AuditEvent> {
    Ok(AuditEvent {
        id: row.get(0)?,
        event_type: row.get(1)?,
        asset_id: row.get(2)?,
        session_id: row.get(3)?,
        job_run_id: row.get(4)?,
        title: row.get(5)?,
        detail: row.get(6)?,
        severity: row.get(7)?,
        metadata_json: row.get(8)?,
        created_at: row.get(9)?,
    })
}

pub fn map_connection_from_endpoint(
    asset: &HostAsset,
    endpoint: &AccessEndpoint,
    credential_ref: Option<&CredentialRef>,
) -> SshConnection {
    let auth_type = endpoint
        .auth_type
        .clone()
        .or_else(|| credential_ref.map(|credential| match credential.credential_kind.as_str() {
            "sshKey" => "key".to_string(),
            _ => "password".to_string(),
        }))
        .unwrap_or_else(|| "password".to_string());

    let password = credential_ref.and_then(|credential| {
        if credential.credential_kind == "password" {
            credential.secret.clone()
        } else {
            None
        }
    });

    let ssh_key_id = endpoint
        .ssh_key_id
        .or_else(|| credential_ref.and_then(|credential| credential.ssh_key_id));

    SshConnection {
        id: asset.id,
        name: asset.name.clone(),
        host: endpoint.host.clone(),
        port: endpoint.port,
        username: endpoint.username.clone(),
        password,
        auth_type: Some(auth_type),
        ssh_key_id,
        jump_host: endpoint.jump_host.clone(),
        jump_port: endpoint.jump_port,
        jump_username: endpoint.jump_username.clone(),
        jump_password: None,
        group_id: asset.folder_id.or(asset.group_id),
        os_type: Some(asset.platform.clone()),
        key_content: None,
        key_passphrase: None,
    }
}

pub fn resolve_asset_bundle(
    conn: &SqliteConnection,
    asset_id: i64,
    endpoint_id: Option<i64>,
) -> Result<(HostAsset, AccessEndpoint, Option<CredentialRef>), String> {
    let asset = conn
        .query_row(
            "SELECT id, name, host, port, platform, folder_id, env_id, labels_csv, owner, criticality,
                    default_workspace_path, access_endpoint_id, bastion_chain_id, health_summary, last_accessed_at,
                    is_favorite, platform, folder_id
             FROM host_assets WHERE id = ?1",
            params![asset_id],
            map_host_asset_row,
        )
        .map_err(|e| e.to_string())?;

    let resolved_endpoint_id = endpoint_id
        .or(asset.access_endpoint_id)
        .ok_or_else(|| "Asset has no default access endpoint".to_string())?;

    let endpoint = conn
        .query_row(
            "SELECT id, asset_id, name, host, port, username, auth_type, credential_ref_id, ssh_key_id, jump_host, jump_port, jump_username, jump_password
             FROM access_endpoints WHERE id = ?1 AND asset_id = ?2",
            params![resolved_endpoint_id, asset_id],
            map_access_endpoint_row,
        )
        .map_err(|e| e.to_string())?;

    let credential_ref = if let Some(credential_ref_id) = endpoint.credential_ref_id {
        Some(
            conn.query_row(
                "SELECT id, name, credential_kind, username, secret, ssh_key_id, asset_id, created_at, updated_at
                 FROM credential_refs WHERE id = ?1",
                params![credential_ref_id],
                map_credential_ref_row,
            )
            .map_err(|e| e.to_string())?,
        )
    } else {
        None
    };

    Ok((asset, endpoint, credential_ref))
}

fn save_asset_bundle(
    tx: &SqliteConnection,
    existing_asset_id: Option<i64>,
    payload: AssetUpsertPayload,
) -> Result<(i64, HostAsset), String> {
    let AssetUpsertPayload {
        mut asset,
        mut default_access_endpoint,
        default_credential_ref,
    } = payload;

    let asset_id = existing_asset_id.unwrap_or_else(|| asset.id.unwrap_or_default());
    let timestamp = now_ts();
    let labels_csv = join_labels(&asset.labels);
    let folder_id = asset.folder_id.or(asset.group_id);
    let endpoint_username = default_access_endpoint.username.clone();
    let endpoint_auth_type = default_access_endpoint
        .auth_type
        .clone()
        .or_else(|| {
            default_credential_ref.as_ref().map(|credential_ref| {
                if credential_ref.credential_kind == "sshKey" {
                    "key".to_string()
                } else {
                    "password".to_string()
                }
            })
        })
        .unwrap_or_else(|| "password".to_string());
    let endpoint_password = if endpoint_auth_type == "password" {
        default_credential_ref
            .as_ref()
            .and_then(|credential_ref| normalize_optional_string(credential_ref.secret.clone()))
    } else {
        None
    };
    let endpoint_ssh_key_id = default_access_endpoint
        .ssh_key_id
        .or_else(|| default_credential_ref.as_ref().and_then(|credential_ref| credential_ref.ssh_key_id));

    tx.execute(
        "INSERT INTO host_assets (
            id, name, host, port, username, password, auth_type, ssh_key_id, jump_host, jump_port, jump_username, jump_password,
            platform, folder_id, env_id, labels_csv, owner, criticality, default_workspace_path,
            access_endpoint_id, bastion_chain_id, health_summary, last_accessed_at, is_favorite, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, NULL, ?12, ?13, ?14, ?15, ?16, ?17, ?18, NULL, ?19, ?20, ?21, ?22, ?23, ?24)
         ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            host = excluded.host,
            port = excluded.port,
            username = excluded.username,
            password = excluded.password,
            auth_type = excluded.auth_type,
            ssh_key_id = excluded.ssh_key_id,
            jump_host = excluded.jump_host,
            jump_port = excluded.jump_port,
            jump_username = excluded.jump_username,
            platform = excluded.platform,
            folder_id = excluded.folder_id,
            env_id = excluded.env_id,
            labels_csv = excluded.labels_csv,
            owner = excluded.owner,
            criticality = excluded.criticality,
            default_workspace_path = excluded.default_workspace_path,
            bastion_chain_id = excluded.bastion_chain_id,
            health_summary = excluded.health_summary,
            last_accessed_at = excluded.last_accessed_at,
            is_favorite = excluded.is_favorite,
            updated_at = excluded.updated_at",
        params![
            asset_id,
            asset.name,
            asset.host,
            asset.port,
            endpoint_username,
            endpoint_password,
            endpoint_auth_type,
            endpoint_ssh_key_id,
            normalize_optional_string(default_access_endpoint.jump_host.clone()),
            default_access_endpoint.jump_port,
            normalize_optional_string(default_access_endpoint.jump_username.clone()),
            asset.platform,
            folder_id,
            asset.env_id,
            labels_csv,
            normalize_optional_string(asset.owner.clone()),
            asset.criticality,
            normalize_optional_string(asset.default_workspace_path.clone()),
            normalize_optional_string(asset.bastion_chain_id.clone()),
            normalize_optional_string(asset.health_summary.clone()),
            asset.last_accessed_at,
            asset.is_favorite.unwrap_or(false) as i64,
            timestamp,
            timestamp
        ],
    )
    .map_err(|e| e.to_string())?;

    upsert_asset_tags(tx, asset_id, &asset.labels)?;

    let credential_ref_id = if let Some(mut credential_ref) = default_credential_ref {
        let credential_id = credential_ref.id.unwrap_or_default();
        let created_at = if existing_asset_id.is_some() && credential_id != 0 {
            credential_ref.created_at
        } else {
            timestamp
        };
        let credential_name = if credential_ref.name.trim().is_empty() {
            default_credential_name(&asset.name, credential_ref.credential_kind.as_str())
        } else {
            credential_ref.name.clone()
        };

        tx.execute(
            "INSERT INTO credential_refs (id, name, credential_kind, username, secret, ssh_key_id, asset_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                credential_kind = excluded.credential_kind,
                username = excluded.username,
                secret = excluded.secret,
                ssh_key_id = excluded.ssh_key_id,
                asset_id = excluded.asset_id,
                updated_at = excluded.updated_at",
            params![
                credential_id,
                credential_name,
                credential_ref.credential_kind,
                normalize_optional_string(credential_ref.username.clone()),
                normalize_optional_string(credential_ref.secret.clone()),
                credential_ref.ssh_key_id,
                Some(asset_id),
                created_at,
                timestamp
            ],
        )
        .map_err(|e| e.to_string())?;

        if credential_id == 0 {
            Some(tx.last_insert_rowid())
        } else {
            Some(credential_id)
        }
    } else {
        None
    };

    let endpoint_id = default_access_endpoint.id.unwrap_or(asset_id);
    let endpoint_name = if default_access_endpoint.name.trim().is_empty() {
        format!("{} default endpoint", asset.name)
    } else {
        default_access_endpoint.name.clone()
    };
    tx.execute(
        "INSERT INTO access_endpoints (id, asset_id, name, host, port, username, auth_type, credential_ref_id, ssh_key_id, jump_host, jump_port, jump_username, jump_password)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, NULL)
         ON CONFLICT(id) DO UPDATE SET
            asset_id = excluded.asset_id,
            name = excluded.name,
            host = excluded.host,
            port = excluded.port,
            username = excluded.username,
            auth_type = excluded.auth_type,
            credential_ref_id = excluded.credential_ref_id,
            ssh_key_id = excluded.ssh_key_id,
            jump_host = excluded.jump_host,
            jump_port = excluded.jump_port,
            jump_username = excluded.jump_username,
            jump_password = NULL",
        params![
            endpoint_id,
            asset_id,
            endpoint_name,
            default_access_endpoint.host,
            default_access_endpoint.port,
            default_access_endpoint.username,
            Some(endpoint_auth_type),
            credential_ref_id,
            endpoint_ssh_key_id,
            normalize_optional_string(default_access_endpoint.jump_host.clone()),
            default_access_endpoint.jump_port,
            normalize_optional_string(default_access_endpoint.jump_username.clone())
        ],
    )
    .map_err(|e| e.to_string())?;

    tx.execute(
        "UPDATE host_assets SET access_endpoint_id = ?2, updated_at = ?3 WHERE id = ?1",
        params![asset_id, endpoint_id, timestamp],
    )
    .map_err(|e| e.to_string())?;

    asset.id = Some(asset_id);
    asset.access_endpoint_id = Some(endpoint_id);
    asset.folder_id = folder_id;
    asset.group_id = folder_id;
    asset.owner = normalize_optional_string(asset.owner);
    asset.default_workspace_path = normalize_optional_string(asset.default_workspace_path);
    asset.bastion_chain_id = normalize_optional_string(asset.bastion_chain_id);
    asset.health_summary = normalize_optional_string(asset.health_summary);
    asset.is_favorite = Some(asset.is_favorite.unwrap_or(false));

    Ok((asset_id, asset))
}

#[tauri::command]
pub fn asset_get_host_assets(app_handle: AppHandle) -> Result<Vec<HostAsset>, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, name, host, port, platform, folder_id, env_id, labels_csv, owner, criticality,
                    default_workspace_path, access_endpoint_id, bastion_chain_id, health_summary, last_accessed_at, is_favorite, folder_id
             FROM host_assets
             ORDER BY COALESCE(last_accessed_at, 0) DESC, name COLLATE NOCASE ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], map_host_asset_row)
        .map_err(|e| e.to_string())?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| e.to_string())?);
    }
    Ok(items)
}

#[tauri::command]
pub fn asset_search_host_assets(app_handle: AppHandle, query: String) -> Result<Vec<HostAsset>, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let pattern = format!("%{}%", query.trim());
    let mut stmt = conn
        .prepare(
            "SELECT id, name, host, port, platform, folder_id, env_id, labels_csv, owner, criticality,
                    default_workspace_path, access_endpoint_id, bastion_chain_id, health_summary, last_accessed_at, is_favorite, folder_id
             FROM host_assets
             WHERE name LIKE ?1 OR host LIKE ?1 OR owner LIKE ?1 OR labels_csv LIKE ?1
             ORDER BY name COLLATE NOCASE ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![pattern], map_host_asset_row)
        .map_err(|e| e.to_string())?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| e.to_string())?);
    }
    Ok(items)
}

#[tauri::command]
pub fn asset_get_asset_folders(app_handle: AppHandle) -> Result<Vec<AssetFolder>, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, name, parent_id, color FROM asset_folders ORDER BY name COLLATE NOCASE ASC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], map_folder_row)
        .map_err(|e| e.to_string())?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| e.to_string())?);
    }
    Ok(items)
}

#[tauri::command]
pub fn asset_get_environments(app_handle: AppHandle) -> Result<Vec<Environment>, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, name, slug, color, description FROM environments ORDER BY name COLLATE NOCASE ASC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], map_environment_row)
        .map_err(|e| e.to_string())?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| e.to_string())?);
    }
    Ok(items)
}

#[tauri::command]
pub fn asset_get_asset_tags(app_handle: AppHandle) -> Result<Vec<AssetTag>, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, name, color FROM asset_tags ORDER BY name COLLATE NOCASE ASC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], map_tag_row)
        .map_err(|e| e.to_string())?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| e.to_string())?);
    }
    Ok(items)
}

#[tauri::command]
pub fn asset_get_saved_views(app_handle: AppHandle) -> Result<Vec<SavedAssetView>, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, name, query_json, created_at, updated_at FROM saved_views ORDER BY updated_at DESC, name COLLATE NOCASE ASC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], map_saved_view_row)
        .map_err(|e| e.to_string())?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| e.to_string())?);
    }
    Ok(items)
}

#[tauri::command]
pub fn access_get_access_endpoints(
    app_handle: AppHandle,
    asset_id: Option<i64>,
) -> Result<Vec<AccessEndpoint>, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let (sql, params_vec): (&str, Vec<i64>) = if let Some(asset_id) = asset_id {
        (
            "SELECT id, asset_id, name, host, port, username, auth_type, credential_ref_id, ssh_key_id, jump_host, jump_port, jump_username, jump_password
             FROM access_endpoints WHERE asset_id = ?1 ORDER BY id ASC",
            vec![asset_id],
        )
    } else {
        (
            "SELECT id, asset_id, name, host, port, username, auth_type, credential_ref_id, ssh_key_id, jump_host, jump_port, jump_username, jump_password
             FROM access_endpoints ORDER BY asset_id ASC, id ASC",
            Vec::new(),
        )
    };

    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows = if params_vec.is_empty() {
        stmt.query_map([], map_access_endpoint_row)
            .map_err(|e| e.to_string())?
    } else {
        stmt.query_map(params![params_vec[0]], map_access_endpoint_row)
            .map_err(|e| e.to_string())?
    };

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| e.to_string())?);
    }
    Ok(items)
}

#[tauri::command]
pub fn access_get_credential_refs(app_handle: AppHandle) -> Result<Vec<CredentialRef>, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, name, credential_kind, username, secret, ssh_key_id, asset_id, created_at, updated_at
             FROM credential_refs
             ORDER BY updated_at DESC, name COLLATE NOCASE ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], map_credential_ref_row)
        .map_err(|e| e.to_string())?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| e.to_string())?);
    }
    Ok(items)
}

#[tauri::command]
pub fn access_create_access_endpoint(
    app_handle: AppHandle,
    endpoint: AccessEndpoint,
) -> Result<AccessEndpoint, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO access_endpoints (asset_id, name, host, port, username, auth_type, credential_ref_id, ssh_key_id, jump_host, jump_port, jump_username, jump_password)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, NULL)",
        params![
            endpoint.asset_id,
            endpoint.name,
            endpoint.host,
            endpoint.port,
            endpoint.username,
            endpoint.auth_type,
            endpoint.credential_ref_id,
            endpoint.ssh_key_id,
            normalize_optional_string(endpoint.jump_host.clone()),
            endpoint.jump_port,
            normalize_optional_string(endpoint.jump_username.clone())
        ],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    conn.execute(
        "UPDATE host_assets SET access_endpoint_id = COALESCE(access_endpoint_id, ?2), updated_at = ?3 WHERE id = ?1",
        params![endpoint.asset_id, id, now_ts()],
    )
    .map_err(|e| e.to_string())?;
    append_audit_event(
        &app_handle,
        "access.endpointCreated",
        Some(endpoint.asset_id),
        None,
        None,
        "Created access endpoint",
        Some(endpoint.name.as_str()),
        "info",
        None,
    )?;
    Ok(AccessEndpoint { id: Some(id), ..endpoint })
}

#[tauri::command]
pub fn access_update_access_endpoint(
    app_handle: AppHandle,
    endpoint: AccessEndpoint,
) -> Result<AccessEndpoint, String> {
    let endpoint_id = endpoint
        .id
        .ok_or_else(|| "Endpoint ID is required".to_string())?;
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE access_endpoints
         SET asset_id = ?1, name = ?2, host = ?3, port = ?4, username = ?5, auth_type = ?6, credential_ref_id = ?7, ssh_key_id = ?8,
             jump_host = ?9, jump_port = ?10, jump_username = ?11, jump_password = NULL
         WHERE id = ?12",
        params![
            endpoint.asset_id,
            endpoint.name,
            endpoint.host,
            endpoint.port,
            endpoint.username,
            endpoint.auth_type,
            endpoint.credential_ref_id,
            endpoint.ssh_key_id,
            normalize_optional_string(endpoint.jump_host.clone()),
            endpoint.jump_port,
            normalize_optional_string(endpoint.jump_username.clone()),
            endpoint_id
        ],
    )
    .map_err(|e| e.to_string())?;
    append_audit_event(
        &app_handle,
        "access.endpointUpdated",
        Some(endpoint.asset_id),
        None,
        None,
        "Updated access endpoint",
        Some(endpoint.name.as_str()),
        "info",
        None,
    )?;
    Ok(endpoint)
}

#[tauri::command]
pub fn access_delete_access_endpoint(app_handle: AppHandle, id: i64) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let asset_id: i64 = conn
        .query_row(
            "SELECT asset_id FROM access_endpoints WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM access_endpoints WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE host_assets
         SET access_endpoint_id = (
            SELECT ae.id FROM access_endpoints ae WHERE ae.asset_id = ?2 ORDER BY ae.id ASC LIMIT 1
         ),
         updated_at = ?3
         WHERE id = ?1",
        params![asset_id, asset_id, now_ts()],
    )
    .map_err(|e| e.to_string())?;
    append_audit_event(
        &app_handle,
        "access.endpointDeleted",
        Some(asset_id),
        None,
        None,
        "Deleted access endpoint",
        None,
        "warning",
        None,
    )?;
    Ok(())
}

#[tauri::command]
pub fn access_create_credential_ref(
    app_handle: AppHandle,
    credential_ref: CredentialRef,
) -> Result<CredentialRef, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let created_at = if credential_ref.created_at == 0 {
        now_ts()
    } else {
        credential_ref.created_at
    };
    let updated_at = now_ts();
    conn.execute(
        "INSERT INTO credential_refs (name, credential_kind, username, secret, ssh_key_id, asset_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            credential_ref.name,
            credential_ref.credential_kind,
            normalize_optional_string(credential_ref.username.clone()),
            normalize_optional_string(credential_ref.secret.clone()),
            credential_ref.ssh_key_id,
            credential_ref.asset_id,
            created_at,
            updated_at
        ],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    append_audit_event(
        &app_handle,
        "access.credentialCreated",
        credential_ref.asset_id,
        None,
        None,
        "Created credential reference",
        Some(credential_ref.name.as_str()),
        "info",
        None,
    )?;
    Ok(CredentialRef {
        id: Some(id),
        created_at,
        updated_at,
        ..credential_ref
    })
}

#[tauri::command]
pub fn access_update_credential_ref(
    app_handle: AppHandle,
    credential_ref: CredentialRef,
) -> Result<CredentialRef, String> {
    let id = credential_ref
        .id
        .ok_or_else(|| "Credential ref ID is required".to_string())?;
    let updated_at = now_ts();
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE credential_refs
         SET name = ?1, credential_kind = ?2, username = ?3, secret = ?4, ssh_key_id = ?5, asset_id = ?6, updated_at = ?7
         WHERE id = ?8",
        params![
            credential_ref.name,
            credential_ref.credential_kind,
            normalize_optional_string(credential_ref.username.clone()),
            normalize_optional_string(credential_ref.secret.clone()),
            credential_ref.ssh_key_id,
            credential_ref.asset_id,
            updated_at,
            id
        ],
    )
    .map_err(|e| e.to_string())?;
    append_audit_event(
        &app_handle,
        "access.credentialUpdated",
        credential_ref.asset_id,
        None,
        None,
        "Updated credential reference",
        Some(credential_ref.name.as_str()),
        "info",
        None,
    )?;
    Ok(CredentialRef { updated_at, ..credential_ref })
}

#[tauri::command]
pub fn access_delete_credential_ref(app_handle: AppHandle, id: i64) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let asset_id = conn
        .query_row(
            "SELECT asset_id FROM credential_refs WHERE id = ?1",
            params![id],
            |row| row.get::<_, Option<i64>>(0),
        )
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM credential_refs WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    append_audit_event(
        &app_handle,
        "access.credentialDeleted",
        asset_id,
        None,
        None,
        "Deleted credential reference",
        None,
        "warning",
        None,
    )?;
    Ok(())
}

#[tauri::command]
pub fn asset_create_host_asset(
    app_handle: AppHandle,
    payload: AssetUpsertPayload,
) -> Result<HostAsset, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO host_assets (
            name, host, port, platform, folder_id, env_id, labels_csv, owner, criticality, default_workspace_path,
            access_endpoint_id, bastion_chain_id, health_summary, last_accessed_at, is_favorite, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, '', NULL, 'medium', NULL, NULL, NULL, NULL, NULL, 0, ?7, ?7)",
        params![
            payload.asset.name,
            payload.asset.host,
            payload.asset.port,
            payload.asset.platform,
            payload.asset.folder_id.or(payload.asset.group_id),
            payload.asset.env_id,
            now_ts()
        ],
    )
    .map_err(|e| e.to_string())?;
    let asset_id = tx.last_insert_rowid();
    let (_, saved_asset) = save_asset_bundle(
        &tx,
        Some(asset_id),
        AssetUpsertPayload {
            asset: HostAsset {
                id: Some(asset_id),
                ..payload.asset
            },
            default_access_endpoint: AccessEndpoint {
                id: payload.default_access_endpoint.id.or(Some(asset_id)),
                asset_id,
                ..payload.default_access_endpoint
            },
            default_credential_ref: payload.default_credential_ref,
        },
    )?;

    append_audit_event_with_conn(
        &tx,
        "asset.created",
        Some(asset_id),
        None,
        None,
        "Created host asset",
        Some("Host asset created from asset center."),
        "info",
        None,
    )?;

    tx.commit().map_err(|e| e.to_string())?;
    Ok(saved_asset)
}

#[tauri::command]
pub fn asset_update_host_asset(
    app_handle: AppHandle,
    payload: AssetUpsertPayload,
) -> Result<HostAsset, String> {
    let asset_id = payload
        .asset
        .id
        .ok_or_else(|| "Asset ID is required".to_string())?;
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let (_, saved_asset) = save_asset_bundle(&tx, Some(asset_id), payload)?;

    append_audit_event_with_conn(
        &tx,
        "asset.updated",
        Some(asset_id),
        None,
        None,
        "Updated host asset",
        Some("Host asset updated from asset center."),
        "info",
        None,
    )?;

    tx.commit().map_err(|e| e.to_string())?;
    Ok(saved_asset)
}

#[tauri::command]
pub fn asset_delete_host_asset(app_handle: AppHandle, id: i64) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    append_audit_event_with_conn(
        &tx,
        "asset.deleted",
        Some(id),
        None,
        None,
        "Deleted host asset",
        Some("Host asset deleted from asset center."),
        "warning",
        None,
    )?;
    tx.execute("DELETE FROM host_assets WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn asset_touch_host_asset(app_handle: AppHandle, id: i64) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE host_assets SET last_accessed_at = ?2, updated_at = ?2 WHERE id = ?1",
        params![id, now_ts()],
    )
    .map_err(|e| e.to_string())?;
    append_audit_event(
        &app_handle,
        "asset.accessed",
        Some(id),
        None,
        None,
        "Asset accessed",
        Some("Asset session entry recorded."),
        "info",
        None,
    )?;
    Ok(())
}

#[tauri::command]
pub fn asset_toggle_favorite(app_handle: AppHandle, id: i64, is_favorite: bool) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE host_assets SET is_favorite = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, is_favorite as i64, now_ts()],
    )
    .map_err(|e| e.to_string())?;
    append_audit_event(
        &app_handle,
        "asset.favoriteChanged",
        Some(id),
        None,
        None,
        if is_favorite {
            "Asset added to favorites"
        } else {
            "Asset removed from favorites"
        },
        None,
        "info",
        None,
    )?;
    Ok(())
}

#[tauri::command]
pub fn asset_create_asset_folder(app_handle: AppHandle, folder: AssetFolder) -> Result<AssetFolder, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO connection_groups (name, parent_id) VALUES (?1, ?2)",
        params![folder.name, folder.parent_id],
    )
    .map_err(|e| e.to_string())?;
    let folder_id = tx.last_insert_rowid();
    tx.execute(
        "INSERT INTO asset_folders (id, name, parent_id, color) VALUES (?1, ?2, ?3, ?4)",
        params![folder_id, folder.name, folder.parent_id, folder.color],
    )
    .map_err(|e| e.to_string())?;
    append_audit_event_with_conn(
        &tx,
        "folder.created",
        None,
        None,
        None,
        "Created asset folder",
        Some("Asset folder created from asset center."),
        "info",
        None,
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(AssetFolder {
        id: Some(folder_id),
        ..folder
    })
}

#[tauri::command]
pub fn asset_update_asset_folder(app_handle: AppHandle, folder: AssetFolder) -> Result<(), String> {
    let folder_id = folder.id.ok_or_else(|| "Folder ID is required".to_string())?;
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "UPDATE connection_groups SET name = ?1, parent_id = ?2 WHERE id = ?3",
        params![folder.name, folder.parent_id, folder_id],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "UPDATE asset_folders SET name = ?1, parent_id = ?2, color = ?3 WHERE id = ?4",
        params![folder.name, folder.parent_id, folder.color, folder_id],
    )
    .map_err(|e| e.to_string())?;
    append_audit_event_with_conn(
        &tx,
        "folder.updated",
        None,
        None,
        None,
        "Updated asset folder",
        None,
        "info",
        None,
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn asset_delete_asset_folder(app_handle: AppHandle, id: i64) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    append_audit_event_with_conn(
        &tx,
        "folder.deleted",
        None,
        None,
        None,
        "Deleted asset folder",
        None,
        "warning",
        None,
    )?;
    tx.execute("DELETE FROM asset_folders WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM connection_groups WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn asset_create_environment(app_handle: AppHandle, environment: Environment) -> Result<Environment, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO environments (name, slug, color, description) VALUES (?1, ?2, ?3, ?4)",
        params![
            environment.name,
            if environment.slug.trim().is_empty() {
                slugify(&environment.name)
            } else {
                environment.slug.clone()
            },
            environment.color,
            environment.description
        ],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    append_audit_event(
        &app_handle,
        "environment.created",
        None,
        None,
        None,
        "Created environment",
        Some(environment.name.as_str()),
        "info",
        None,
    )?;
    Ok(Environment {
        id: Some(id),
        slug: if environment.slug.trim().is_empty() {
            slugify(&environment.name)
        } else {
            environment.slug
        },
        ..environment
    })
}

#[tauri::command]
pub fn asset_update_environment(app_handle: AppHandle, environment: Environment) -> Result<(), String> {
    let id = environment.id.ok_or_else(|| "Environment ID is required".to_string())?;
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE environments SET name = ?1, slug = ?2, color = ?3, description = ?4 WHERE id = ?5",
        params![
            environment.name,
            if environment.slug.trim().is_empty() {
                slugify(&environment.name)
            } else {
                environment.slug
            },
            environment.color,
            environment.description,
            id
        ],
    )
    .map_err(|e| e.to_string())?;
    append_audit_event(
        &app_handle,
        "environment.updated",
        None,
        None,
        None,
        "Updated environment",
        Some(environment.name.as_str()),
        "info",
        None,
    )?;
    Ok(())
}

#[tauri::command]
pub fn asset_delete_environment(app_handle: AppHandle, id: i64) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM environments WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    append_audit_event(
        &app_handle,
        "environment.deleted",
        None,
        None,
        None,
        "Deleted environment",
        None,
        "warning",
        None,
    )?;
    Ok(())
}

#[tauri::command]
pub fn asset_create_asset_tag(app_handle: AppHandle, tag: AssetTag) -> Result<AssetTag, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO asset_tags (name, color) VALUES (?1, ?2)",
        params![tag.name, tag.color],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    append_audit_event(
        &app_handle,
        "tag.created",
        None,
        None,
        None,
        "Created asset tag",
        Some(tag.name.as_str()),
        "info",
        None,
    )?;
    Ok(AssetTag { id: Some(id), ..tag })
}

#[tauri::command]
pub fn asset_delete_asset_tag(app_handle: AppHandle, id: i64) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM asset_tags WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    append_audit_event(
        &app_handle,
        "tag.deleted",
        None,
        None,
        None,
        "Deleted asset tag",
        None,
        "warning",
        None,
    )?;
    Ok(())
}

#[tauri::command]
pub fn asset_create_saved_view(app_handle: AppHandle, view: SavedAssetView) -> Result<SavedAssetView, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let timestamp = now_ts();
    conn.execute(
        "INSERT INTO saved_views (name, query_json, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
        params![view.name, view.query_json, timestamp, timestamp],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    Ok(SavedAssetView {
        id: Some(id),
        created_at: timestamp,
        updated_at: timestamp,
        ..view
    })
}

#[tauri::command]
pub fn asset_delete_saved_view(app_handle: AppHandle, id: i64) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM saved_views WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn ops_list_job_templates(app_handle: AppHandle) -> Result<Vec<JobTemplate>, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, name, command, scope_type, scope_value, risk_level, requires_confirmation, created_at, updated_at
             FROM job_templates
             ORDER BY updated_at DESC, name COLLATE NOCASE ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], map_job_template_row)
        .map_err(|e| e.to_string())?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| e.to_string())?);
    }
    Ok(items)
}

#[tauri::command]
pub fn ops_create_job_template(app_handle: AppHandle, template: JobTemplate) -> Result<JobTemplate, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let timestamp = now_ts();
    conn.execute(
        "INSERT INTO job_templates (name, command, scope_type, scope_value, risk_level, requires_confirmation, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            template.name,
            template.command,
            template.scope_type,
            template.scope_value,
            template.risk_level,
            template.requires_confirmation as i64,
            timestamp,
            timestamp
        ],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    append_audit_event(
        &app_handle,
        "jobTemplate.created",
        None,
        None,
        None,
        "Created job template",
        Some(template.name.as_str()),
        "info",
        None,
    )?;
    Ok(JobTemplate {
        id: Some(id),
        created_at: timestamp,
        updated_at: timestamp,
        ..template
    })
}

#[tauri::command]
pub fn ops_delete_job_template(app_handle: AppHandle, id: i64) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM job_templates WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    append_audit_event(
        &app_handle,
        "jobTemplate.deleted",
        None,
        None,
        None,
        "Deleted job template",
        None,
        "warning",
        None,
    )?;
    Ok(())
}

#[tauri::command]
pub fn ops_list_job_runs(app_handle: AppHandle, asset_id: Option<i64>) -> Result<Vec<JobRun>, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let (sql, param): (&str, Option<i64>) = if let Some(asset_id) = asset_id {
        (
            "SELECT id, asset_id, session_id, template_id, command, status, output, risk_level, initiated_by, source, created_at, completed_at
             FROM job_runs WHERE asset_id = ?1 ORDER BY created_at DESC",
            Some(asset_id),
        )
    } else {
        (
            "SELECT id, asset_id, session_id, template_id, command, status, output, risk_level, initiated_by, source, created_at, completed_at
             FROM job_runs ORDER BY created_at DESC",
            None,
        )
    };
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows = if let Some(asset_id) = param {
        stmt.query_map(params![asset_id], map_job_run_row)
            .map_err(|e| e.to_string())?
    } else {
        stmt.query_map([], map_job_run_row)
            .map_err(|e| e.to_string())?
    };

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| e.to_string())?);
    }
    Ok(items)
}

#[tauri::command]
pub async fn ops_execute_job(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
    asset_id: Option<i64>,
    command_text: String,
    risk_level: Option<String>,
    source: Option<String>,
) -> Result<JobRun, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let created_at = now_ts();
    conn.execute(
        "INSERT INTO job_runs (asset_id, session_id, command, status, output, risk_level, initiated_by, source, created_at, completed_at)
         VALUES (?1, ?2, ?3, 'running', NULL, ?4, 'local-user', ?5, ?6, NULL)",
        params![
            asset_id,
            session_id,
            command_text,
            risk_level.clone().unwrap_or_else(|| "medium".to_string()),
            source.clone(),
            created_at
        ],
    )
    .map_err(|e| e.to_string())?;
    let job_run_id = conn.last_insert_rowid();
    drop(conn);

    append_audit_event(
        &app_handle,
        "job.started",
        asset_id,
        Some(session_id.as_str()),
        Some(job_run_id),
        "Started job execution",
        Some(command_text.as_str()),
        "info",
        None,
    )?;

    let output = command::exec_command(
        app_handle.clone(),
        state,
        session_id.clone(),
        command_text.clone(),
        None,
    )
    .await;

    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    match output {
        Ok(result) => {
            conn.execute(
                "UPDATE job_runs SET status = 'completed', output = ?2, completed_at = ?3 WHERE id = ?1",
                params![job_run_id, result, now_ts()],
            )
            .map_err(|e| e.to_string())?;
            append_audit_event(
                &app_handle,
                "job.completed",
                asset_id,
                Some(session_id.as_str()),
                Some(job_run_id),
                "Completed job execution",
                Some(command_text.as_str()),
                "info",
                None,
            )?;
        }
        Err(error) => {
            conn.execute(
                "UPDATE job_runs SET status = 'error', output = ?2, completed_at = ?3 WHERE id = ?1",
                params![job_run_id, error, now_ts()],
            )
            .map_err(|e| e.to_string())?;
            append_audit_event(
                &app_handle,
                "job.failed",
                asset_id,
                Some(session_id.as_str()),
                Some(job_run_id),
                "Job execution failed",
                Some(command_text.as_str()),
                "warning",
                None,
            )?;
        }
    }

    conn.query_row(
        "SELECT id, asset_id, session_id, template_id, command, status, output, risk_level, initiated_by, source, created_at, completed_at
         FROM job_runs WHERE id = ?1",
        params![job_run_id],
        map_job_run_row,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn audit_list_events(
    app_handle: AppHandle,
    asset_id: Option<i64>,
    limit: Option<usize>,
) -> Result<Vec<AuditEvent>, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let limit = limit.unwrap_or(200) as i64;
    let (sql, params_asset): (&str, Option<i64>) = if let Some(asset_id) = asset_id {
        (
            "SELECT id, event_type, asset_id, session_id, job_run_id, title, detail, severity, metadata_json, created_at
             FROM audit_events WHERE asset_id = ?1 ORDER BY created_at DESC LIMIT ?2",
            Some(asset_id),
        )
    } else {
        (
            "SELECT id, event_type, asset_id, session_id, job_run_id, title, detail, severity, metadata_json, created_at
             FROM audit_events ORDER BY created_at DESC LIMIT ?1",
            None,
        )
    };

    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows = if let Some(asset_id) = params_asset {
        stmt.query_map(params![asset_id, limit], map_audit_event_row)
            .map_err(|e| e.to_string())?
    } else {
        stmt.query_map(params![limit], map_audit_event_row)
            .map_err(|e| e.to_string())?
    };
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| e.to_string())?);
    }
    Ok(items)
}

#[tauri::command]
pub fn audit_create_event(app_handle: AppHandle, event: AuditEvent) -> Result<AuditEvent, String> {
    let id = append_audit_event(
        &app_handle,
        event.event_type.as_str(),
        event.asset_id,
        event.session_id.as_deref(),
        event.job_run_id,
        event.title.as_str(),
        event.detail.as_deref(),
        event.severity.as_str(),
        event.metadata_json.as_deref(),
    )?;
    Ok(AuditEvent {
        id: Some(id),
        created_at: now_ts(),
        ..event
    })
}

#[tauri::command]
pub fn sync_get_state(app_handle: AppHandle) -> Result<SyncState, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT id, state_key, status, version, endpoint_url, last_synced_at, last_error, metadata_json, updated_at
         FROM sync_state ORDER BY id ASC LIMIT 1",
        [],
        |row| {
            Ok(SyncState {
                id: row.get(0)?,
                state_key: row.get(1)?,
                status: row.get(2)?,
                version: row.get(3)?,
                endpoint_url: row.get(4)?,
                last_synced_at: row.get(5)?,
                last_error: row.get(6)?,
                metadata_json: row.get(7)?,
                updated_at: row.get(8)?,
            })
        },
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn sync_save_state(app_handle: AppHandle, state: SyncState) -> Result<SyncState, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let updated_at = now_ts();
    conn.execute(
        "INSERT INTO sync_state (id, state_key, status, version, endpoint_url, last_synced_at, last_error, metadata_json, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
         ON CONFLICT(state_key) DO UPDATE SET
            status = excluded.status,
            version = excluded.version,
            endpoint_url = excluded.endpoint_url,
            last_synced_at = excluded.last_synced_at,
            last_error = excluded.last_error,
            metadata_json = excluded.metadata_json,
            updated_at = excluded.updated_at",
        params![
            state.id.unwrap_or(1),
            state.state_key,
            state.status,
            state.version,
            state.endpoint_url,
            state.last_synced_at,
            state.last_error,
            state.metadata_json,
            updated_at
        ],
    )
    .map_err(|e| e.to_string())?;
    sync_get_state(app_handle)
}

#[tauri::command]
pub fn ai_plan_action(asset: HostAsset, user_request: String) -> Result<String, String> {
    Ok(format!(
        "Plan action for asset '{}' ({}): inspect current state, identify safe commands, require confirmation before write operations. Request: {}",
        asset.name, asset.host, user_request
    ))
}

#[tauri::command]
pub fn ai_explain_state(asset: HostAsset, observed_state: String) -> Result<String, String> {
    Ok(format!(
        "Asset '{}' on platform '{}' is currently observed as: {}. Explain likely causes, impact, and safe next checks.",
        asset.name, asset.platform, observed_state
    ))
}

#[tauri::command]
pub fn ai_generate_runbook(asset: HostAsset, target: String) -> Result<String, String> {
    Ok(format!(
        "Runbook for asset '{}': define pre-checks, read-only validation, guarded remediation, and rollback notes for {}.",
        asset.name, target
    ))
}

#[tauri::command]
pub async fn session_connect_asset(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    asset_id: i64,
    access_endpoint_id: Option<i64>,
    existing_session_id: Option<String>,
) -> Result<AssetSessionConnectResult, String> {
    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    let (asset, endpoint, credential_ref) = resolve_asset_bundle(&conn, asset_id, access_endpoint_id)?;
    drop(conn);

    let ssh_config = map_connection_from_endpoint(&asset, &endpoint, credential_ref.as_ref());
    let session_id = client::connect(
        app_handle.clone(),
        state,
        ssh_config,
        existing_session_id,
    )
    .await?;

    let db_path = get_db_path(&app_handle);
    let conn = SqliteConnection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE host_assets SET last_accessed_at = ?2, updated_at = ?2 WHERE id = ?1",
        params![asset_id, now_ts()],
    )
    .map_err(|e| e.to_string())?;
    append_audit_event(
        &app_handle,
        "session.connected",
        Some(asset_id),
        Some(session_id.as_str()),
        None,
        "Connected asset session",
        Some(asset.name.as_str()),
        "info",
        None,
    )?;

    Ok(AssetSessionConnectResult {
        session_id,
        asset_id,
        asset_name: asset.name,
        env_id: asset.env_id,
        access_endpoint_id: endpoint.id,
        credential_ref_id: endpoint.credential_ref_id.or_else(|| credential_ref.and_then(|item| item.id)),
        bastion_chain_id: asset.bastion_chain_id,
        risk_level: asset.criticality,
        health_summary: asset.health_summary,
        os_info: asset.platform,
    })
}

#[tauri::command]
pub async fn session_disconnect_asset(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
    asset_id: Option<i64>,
) -> Result<(), String> {
    client::disconnect(state, session_id.clone()).await?;
    append_audit_event(
        &app_handle,
        "session.disconnected",
        asset_id,
        Some(session_id.as_str()),
        None,
        "Disconnected asset session",
        None,
        "info",
        None,
    )?;
    Ok(())
}

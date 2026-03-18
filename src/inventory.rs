use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RpcKind {
    Daemon,
    Wallet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RpcField {
    pub name: String,
    pub ty: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RpcMethodSpec {
    pub command: String,
    pub method: String,
    pub request_fields: Vec<RpcField>,
    pub request_summary: String,
    pub request_descriptor: String,
}

#[derive(Debug, Clone)]
pub struct ParamPreset {
    pub key: String,
    pub label: String,
    pub payload: Value,
}

#[derive(Debug, Clone, Default)]
pub struct RpcContext {
    pub current_height: Option<u64>,
    pub current_hash: Option<String>,
    pub wallet_address: Option<String>,
}

pub fn load_inventory(kind: RpcKind) -> Vec<RpcMethodSpec> {
    let raw = match kind {
        RpcKind::Daemon => include_str!("../rpc.output"),
        RpcKind::Wallet => include_str!("../walletrpc.output"),
    };

    parse_inventory(raw)
}

pub fn default_method(kind: RpcKind, methods: &[RpcMethodSpec]) -> Option<String> {
    let preferred: &[&str] = match kind {
        RpcKind::Daemon => &["get_info", "get_version", "get_block_count"],
        RpcKind::Wallet => &["get_version", "getheight", "get_height", "getbalance"],
    };

    preferred
        .iter()
        .find_map(|target| {
            methods
                .iter()
                .find(|spec| spec.method == *target)
                .map(|spec| spec.method.clone())
        })
        .or_else(|| methods.first().map(|spec| spec.method.clone()))
}

pub fn method_names(methods: &[RpcMethodSpec]) -> Vec<String> {
    methods.iter().map(|spec| spec.method.clone()).collect()
}

pub fn daemon_method_names(methods: &[RpcMethodSpec], restricted: bool) -> Vec<String> {
    if !restricted {
        return method_names(methods);
    }

    let allowed = restricted_allowed_daemon_methods();
    methods
        .iter()
        .filter(|spec| allowed.contains(spec.method.as_str()))
        .filter(|spec| !requires_client_signature(spec))
        .map(|spec| spec.method.clone())
        .collect()
}

pub fn daemon_default_method(methods: &[RpcMethodSpec], restricted: bool) -> Option<String> {
    let preferred: &[&str] = if restricted {
        &["get_version", "get_block_count", "getblockcount"]
    } else {
        &["get_info", "get_version", "get_block_count"]
    };
    let options = daemon_method_names(methods, restricted);

    preferred
        .iter()
        .find_map(|target| {
            options
                .iter()
                .find(|method| method.as_str() == *target)
                .cloned()
        })
        .or_else(|| options.first().cloned())
}

pub fn find_method<'a>(methods: &'a [RpcMethodSpec], method: &str) -> Option<&'a RpcMethodSpec> {
    methods.iter().find(|spec| spec.method == method)
}

pub fn is_read_only_method(kind: RpcKind, method: &str) -> bool {
    match kind {
        RpcKind::Daemon => !matches!(
            method,
            "set_bans"
                | "flush_txpool"
                | "relay_tx"
                | "start_mining"
                | "stop_mining"
                | "prune_blockchain"
                | "set_log_level"
                | "set_log_categories"
                | "save_bc"
                | "pop_blocks"
                | "update"
                | "submit_block"
                | "generateblocks"
                | "add_aux_pow"
                | "flush_cache"
        ),
        RpcKind::Wallet => matches!(
            method,
            "audit"
                | "check_reserve_proof"
                | "check_spend_proof"
                | "check_tx_key"
                | "check_tx_proof"
                | "estimate_tx_size_and_weight"
                | "frozen"
                | "get_accounts"
                | "get_account_tags"
                | "get_address"
                | "getaddress"
                | "get_address_book"
                | "get_address_index"
                | "get_attribute"
                | "get_balance"
                | "getbalance"
                | "get_bulk_payments"
                | "get_default_fee_priority"
                | "get_height"
                | "getheight"
                | "get_languages"
                | "get_payments"
                | "get_reserve_proof"
                | "get_transfer_by_txid"
                | "get_transfers"
                | "get_tx_key"
                | "get_tx_notes"
                | "get_version"
                | "incoming_transfers"
                | "is_multisig"
                | "query_key"
                | "validate_address"
                | "verify"
                | "verify_message"
        ),
    }
}

pub fn presets_for_method(
    kind: RpcKind,
    method: &RpcMethodSpec,
    context: &RpcContext,
) -> Vec<ParamPreset> {
    if method.request_fields.is_empty() {
        return vec![ParamPreset {
            key: "empty".into(),
            label: "empty request".into(),
            payload: Value::Null,
        }];
    }

    let daemon_specials = if kind == RpcKind::Daemon {
        daemon_presets(method.method.as_str(), context)
    } else {
        Vec::new()
    };

    if !daemon_specials.is_empty() {
        return daemon_specials;
    }

    let mut payload = Map::new();
    for field in &method.request_fields {
        payload.insert(
            field.name.clone(),
            sample_value(kind, method.method.as_str(), field, context),
        );
    }

    let label = if method.request_summary.is_empty() {
        "generated template".to_string()
    } else {
        truncate_label(&method.request_summary, 84)
    };

    vec![ParamPreset {
        key: "template".into(),
        label,
        payload: Value::Object(payload),
    }]
}

pub fn input_strings_from_payload(
    fields: &[RpcField],
    payload: &Value,
) -> BTreeMap<String, String> {
    let mut values = BTreeMap::new();
    let object = payload.as_object();

    for field in fields {
        let value = object
            .and_then(|object| object.get(&field.name))
            .cloned()
            .unwrap_or(Value::Null);
        values.insert(field.name.clone(), stringify_input_value(&value));
    }

    values
}

pub fn parse_input_value(field: &RpcField, input: &str) -> Result<Value, String> {
    let trimmed = input.trim();

    if trimmed.is_empty() {
        return Ok(empty_value_for_type(&field.ty));
    }

    if looks_like_json(trimmed) {
        return serde_json::from_str(trimmed)
            .map_err(|error| format!("{} must contain valid JSON: {error}", field.name));
    }

    if field.ty.contains("bool") {
        return parse_bool(trimmed)
            .map(Value::Bool)
            .ok_or_else(|| format!("{} expects true or false.", field.name));
    }

    if field.ty.contains("vector") || field.ty.contains("list") || field.ty.contains("set") {
        let items = trimmed
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(|item| scalar_value_for_type(&field.ty, item))
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(Value::Array(items));
    }

    scalar_value_for_type(&field.ty, trimmed)
}

fn daemon_presets(method: &str, context: &RpcContext) -> Vec<ParamPreset> {
    let current_height = context.current_height.unwrap_or(0);
    let current_hash = context.current_hash.clone().unwrap_or_default();
    let recent_start = current_height.saturating_sub(9);
    let recent_heights = vec![
        current_height.saturating_sub(2),
        current_height.saturating_sub(1),
        current_height,
    ];

    match method {
        "get_block" | "getblock" => vec![
            ParamPreset {
                key: "current_height".into(),
                label: "current height".into(),
                payload: json!({ "height": current_height }),
            },
            ParamPreset {
                key: "current_hash".into(),
                label: "current hash".into(),
                payload: json!({ "hash": current_hash }),
            },
        ],
        "get_block_header_by_height" | "getblockheaderbyheight" | "hard_fork_info" => {
            vec![ParamPreset {
                key: "current_height".into(),
                label: "current height".into(),
                payload: json!({ "height": current_height }),
            }]
        }
        "get_block_header_by_hash" | "getblockheaderbyhash" => vec![ParamPreset {
            key: "current_hash".into(),
            label: "current hash".into(),
            payload: json!({ "hash": current_hash }),
        }],
        "get_block_headers_range" | "getblockheadersrange" | "get_blocks" => vec![ParamPreset {
            key: "recent_range".into(),
            label: "recent range".into(),
            payload: json!({ "start_height": recent_start, "end_height": current_height }),
        }],
        "get_blocks_by_height.bin"
        | "getblocks_by_height.bin"
        | "get_hashes.bin"
        | "gethashes.bin" => vec![ParamPreset {
            key: "recent_heights".into(),
            label: "recent heights".into(),
            payload: json!({ "heights": recent_heights }),
        }],
        "get_fee_estimate" => vec![ParamPreset {
            key: "grace_blocks_10".into(),
            label: "grace blocks 10".into(),
            payload: json!({ "grace_blocks": 10 }),
        }],
        "get_coinbase_tx_sum" => {
            let count = current_height.min(100);
            vec![ParamPreset {
                key: "coinbase_window_100".into(),
                label: "coinbase window 100".into(),
                payload: json!({
                    "height": current_height.saturating_sub(count.saturating_sub(1)),
                    "count": count
                }),
            }]
        }
        "get_output_histogram" => vec![ParamPreset {
            key: "histogram_defaults".into(),
            label: "histogram defaults".into(),
            payload: json!({
                "amounts": [0],
                "min_count": 0,
                "max_count": 100,
                "unlocked": false,
                "recent_cutoff": 0
            }),
        }],
        "get_output_distribution" => vec![ParamPreset {
            key: "distribution_recent".into(),
            label: "distribution recent".into(),
            payload: json!({
                "amounts": [0],
                "from_height": 0,
                "to_height": current_height,
                "cumulative": false,
                "binary": false,
                "compress": false
            }),
        }],
        "get_public_nodes" => vec![ParamPreset {
            key: "public_nodes_defaults".into(),
            label: "public nodes defaults".into(),
            payload: json!({
                "gray": true,
                "white": true,
                "include_blocked": false
            }),
        }],
        "get_block_template" | "getblocktemplate" => vec![ParamPreset {
            key: "block_template_stub".into(),
            label: "block template stub".into(),
            payload: json!({
                "wallet_address": "Se1BlockTemplateStubAddressNotForMiningUse",
                "reserve_size": 60
            }),
        }],
        _ => Vec::new(),
    }
}

fn sample_value(kind: RpcKind, method: &str, field: &RpcField, context: &RpcContext) -> Value {
    let name = field.name.as_str();
    let ty = field.ty.as_str();

    match name {
        "height" | "start_height" | "end_height" | "restore_height" => {
            json!(context.current_height.unwrap_or(0))
        }
        "count" => json!(10),
        "account_index" => json!(0),
        "address_index" => json!(0),
        "min_block_height" => json!(0),
        "period" => json!(10),
        "ring_size" => json!(16),
        "n_inputs" | "n_outputs" => json!(1),
        "reserve_size" => json!(60),
        "trusted" => json!(true),
        "strict"
        | "strict_balances"
        | "all_accounts"
        | "all_assets"
        | "clear"
        | "do_not_relay"
        | "get_tx_keys"
        | "get_tx_hex"
        | "get_tx_metadata"
        | "subaddr_indices_all"
        | "autosave_current"
        | "enable"
        | "regexp"
        | "rct"
        | "unlocked"
        | "include_blocked"
        | "gray"
        | "white" => json!(false),
        "hash" => {
            if context
                .current_hash
                .as_deref()
                .unwrap_or_default()
                .is_empty()
            {
                json!("")
            } else {
                json!(context.current_hash.clone().unwrap_or_default())
            }
        }
        "address" | "wallet_address" => {
            if let Some(address) = &context.wallet_address {
                json!(address)
            } else if method == "get_block_template" || method == "getblocktemplate" {
                json!("Se1BlockTemplateStubAddressNotForMiningUse")
            } else {
                json!("")
            }
        }
        "address_indices" | "subaddr_indices" | "entries" | "payment_ids" | "txids" | "hashes" => {
            json!([])
        }
        "amounts" => json!([0]),
        "asset_type" => json!("SAL"),
        "background_sync_type" => json!("off"),
        "filename" => json!("wallet-name"),
        "language" => json!("English"),
        "ssl_support" if kind == RpcKind::Wallet => json!("autodetect"),
        "label"
        | "description"
        | "key"
        | "message"
        | "name"
        | "proxy"
        | "seed"
        | "seed_offset"
        | "signature"
        | "tag"
        | "ticker"
        | "token_metadata_hex"
        | "txid"
        | "tx_key"
        | "unsigned_txset"
        | "multisig_txset"
        | "url"
        | "value"
        | "payment_id"
        | "ssl_support"
        | "ssl_private_key_path"
        | "ssl_certificate_path"
        | "ssl_ca_file" => json!(""),
        "password"
        | "old_password"
        | "new_password"
        | "wallet_password"
        | "background_cache_password"
        | "spendkey"
        | "viewkey"
        | "multisig_info" => {
            if ty.contains("vector") {
                json!([])
            } else {
                json!("")
            }
        }
        "ssl_allowed_fingerprints" => json!([]),
        "ssl_allow_any_cert" => json!(true),
        "carrot" | "cryptonote" => json!(false),
        _ if ty.contains("bool") => json!(false),
        _ if ty.contains("uint") || ty.contains("int") => json!(0),
        _ if ty.contains("vector") || ty.contains("list") || ty.contains("set") => json!([]),
        _ => json!(""),
    }
}

fn stringify_input_value(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(value) => value.clone(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Array(_) | Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
    }
}

fn empty_value_for_type(ty: &str) -> Value {
    if ty.contains("vector") || ty.contains("list") || ty.contains("set") {
        Value::Array(Vec::new())
    } else if ty.contains("bool") {
        Value::Bool(false)
    } else if ty.contains("uint") || ty.contains("int") {
        json!(0)
    } else {
        Value::String(String::new())
    }
}

fn scalar_value_for_type(ty: &str, raw: &str) -> Result<Value, String> {
    if ty.contains("bool") {
        parse_bool(raw)
            .map(Value::Bool)
            .ok_or_else(|| format!("expected true or false, got {raw}"))
    } else if ty.contains("uint8")
        || ty.contains("uint16")
        || ty.contains("uint32")
        || ty.contains("uint64")
    {
        raw.parse::<u64>()
            .map(|value| json!(value))
            .map_err(|_| format!("expected unsigned integer, got {raw}"))
    } else if ty.contains("int8")
        || ty.contains("int16")
        || ty.contains("int32")
        || ty.contains("int64")
    {
        raw.parse::<i64>()
            .map(|value| json!(value))
            .map_err(|_| format!("expected integer, got {raw}"))
    } else if ty.contains("double") || ty.contains("float") {
        raw.parse::<f64>()
            .map(|value| json!(value))
            .map_err(|_| format!("expected number, got {raw}"))
    } else {
        Ok(Value::String(raw.to_string()))
    }
}

fn parse_bool(raw: &str) -> Option<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn looks_like_json(input: &str) -> bool {
    input.starts_with('[') || input.starts_with('{') || input == "null" || input.starts_with('"')
}

fn truncate_label(input: &str, max: usize) -> String {
    if input.len() <= max {
        input.to_string()
    } else {
        format!("{}...", &input[..max.saturating_sub(3)])
    }
}

fn parse_inventory(raw: &str) -> Vec<RpcMethodSpec> {
    let mut methods = Vec::new();
    let mut command = String::new();
    let mut json_rpc_methods: Vec<String> = Vec::new();
    let mut request_fields = Vec::new();
    let mut request_summary = String::new();
    let mut request_descriptor = String::new();

    for line in raw.lines() {
        if line.starts_with("COMMAND_RPC_") {
            flush_method(
                &mut methods,
                &command,
                &json_rpc_methods,
                &request_fields,
                &request_summary,
                &request_descriptor,
            );
            command = line.trim().to_string();
            json_rpc_methods.clear();
            request_fields.clear();
            request_summary.clear();
            request_descriptor.clear();
            continue;
        }

        if let Some(value) = line.strip_prefix("json_rpc methods:") {
            json_rpc_methods = value
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty() && *value != "none")
                .map(ToOwned::to_owned)
                .collect();
            continue;
        }

        if let Some(value) = line.strip_prefix("request fields") {
            request_descriptor = line.trim().to_string();
            let summary = value
                .split_once(':')
                .map(|(_, value)| value.trim())
                .unwrap_or_default();

            request_summary = if summary == "none" {
                String::new()
            } else {
                summary.to_string()
            };
            request_fields = parse_fields(summary);
        }
    }

    flush_method(
        &mut methods,
        &command,
        &json_rpc_methods,
        &request_fields,
        &request_summary,
        &request_descriptor,
    );

    methods
}

fn flush_method(
    methods: &mut Vec<RpcMethodSpec>,
    command: &str,
    json_rpc_methods: &[String],
    request_fields: &[RpcField],
    request_summary: &str,
    request_descriptor: &str,
) {
    for method in json_rpc_methods {
        methods.push(RpcMethodSpec {
            command: command.to_string(),
            method: method.clone(),
            request_fields: request_fields.to_vec(),
            request_summary: request_summary.to_string(),
            request_descriptor: request_descriptor.to_string(),
        });
    }
}

fn parse_fields(raw: &str) -> Vec<RpcField> {
    if raw.is_empty() || raw == "none" {
        return Vec::new();
    }

    raw.split(',')
        .filter_map(|entry| {
            let field = entry.trim();
            let (name, ty) = field.split_once(':')?;
            Some(RpcField {
                name: name.trim().to_string(),
                ty: ty.trim().to_string(),
            })
        })
        .collect()
}

fn restricted_allowed_daemon_methods() -> BTreeSet<String> {
    include_str!("../rpc_matches.txt")
        .lines()
        .filter(|line| line.contains("src/rpc/core_rpc_server.h:"))
        .filter_map(|line| {
            let is_json_rpc_map = line.contains("MAP_JON_RPC(")
                || line.contains("MAP_JON_RPC_WE(")
                || line.contains("MAP_JON_RPC_WE_IF(");
            if !is_json_rpc_map {
                return None;
            }

            if line.contains("_IF(") && line.contains("!m_restricted") {
                return None;
            }

            line.split('"').nth(1).map(ToOwned::to_owned)
        })
        .collect()
}

fn requires_client_signature(method: &RpcMethodSpec) -> bool {
    method
        .request_descriptor
        .contains("rpc_access_request_base")
}

#[cfg(test)]
mod tests {
    use super::{RpcKind, daemon_method_names, load_inventory};

    #[test]
    fn restricted_daemon_methods_include_allowed_calls() {
        let methods = load_inventory(RpcKind::Daemon);
        let restricted = daemon_method_names(&methods, true);

        assert!(restricted.iter().any(|method| method == "get_version"));
        assert!(restricted.iter().any(|method| method == "get_block_count"));
        assert!(restricted.iter().any(|method| method == "get_miner_data"));
    }

    #[test]
    fn restricted_daemon_methods_exclude_blocked_calls() {
        let methods = load_inventory(RpcKind::Daemon);
        let restricted = daemon_method_names(&methods, true);

        assert!(!restricted.iter().any(|method| method == "get_connections"));
        assert!(!restricted.iter().any(|method| method == "rpc_access_data"));
        assert!(!restricted.iter().any(|method| method == "get_info"));
        assert!(!restricted.iter().any(|method| method == "rpc_access_info"));
        assert!(
            !restricted
                .iter()
                .any(|method| method == "rpc_access_tracking")
        );
    }
}

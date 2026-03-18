use reqwest::Method;
use reqwest::blocking::{Client, RequestBuilder, Response};
use reqwest::header::{
    ACCEPT, AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue, WWW_AUTHENTICATE,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct RpcConnectionSettings {
    pub endpoints: Vec<String>,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Clone)]
pub struct RpcClient {
    endpoints: Vec<String>,
    http: Client,
    username: Option<String>,
    password: Option<String>,
}

impl RpcClient {
    pub fn new(settings: RpcConnectionSettings) -> Result<Self, String> {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let builder = Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(10))
            .danger_accept_invalid_certs(true);

        let http = builder
            .build()
            .map_err(|err| format!("failed to create HTTP client: {err}"))?;

        Ok(Self {
            endpoints: settings.endpoints,
            http,
            username: settings.username,
            password: settings.password,
        })
    }

    pub fn json_rpc(&self, method: &str, params: Value) -> Result<Value, String> {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": "salvium-monitor",
            "method": method,
            "params": params,
        });

        let mut errors = Vec::new();

        for endpoint in &self.endpoints {
            match self.send_json(Method::POST, endpoint, Some(&payload)) {
                Ok(response) => match Self::decode(response) {
                    Ok(value) => return Ok(value),
                    Err(error) => errors.push(format!("{endpoint}: {error}")),
                },
                Err(error) => errors.push(format!("{endpoint}: {error}")),
            }
        }

        Err(errors.join("; "))
    }

    pub fn call(&self, method: &str, params: Value) -> Result<Value, String> {
        let mut errors = Vec::new();

        for endpoint in &self.endpoints {
            let json_rpc_attempt = self.json_rpc_single(endpoint, method, params.clone());

            if let Ok(value) = &json_rpc_attempt {
                if value.get("error").is_none() {
                    return Ok(value.clone());
                }
            }

            let rest_endpoint = format!("{}/{}", base_endpoint(endpoint), method);
            let rest_attempt = if params.is_null() || is_empty_object(&params) {
                self.send_json(Method::GET, &rest_endpoint, None)
            } else {
                self.send_json(Method::POST, &rest_endpoint, Some(&params))
            }
            .and_then(Self::decode_flexible);

            match (json_rpc_attempt, rest_attempt) {
                (Ok(value), Ok(rest_value)) => {
                    if value.get("error").is_none() {
                        return Ok(value);
                    }
                    return Ok(rest_value);
                }
                (Err(_), Ok(rest_value)) => return Ok(rest_value),
                (Ok(value), Err(_)) => return Ok(value),
                (Err(json_error), Err(rest_error)) => errors.push(format!(
                    "{endpoint}: json_rpc failed: {json_error}; endpoint call failed: {rest_error}"
                )),
            }
        }

        Err(errors.join("; "))
    }

    fn json_rpc_single(
        &self,
        endpoint: &str,
        method: &str,
        params: Value,
    ) -> Result<Value, String> {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": "salvium-monitor",
            "method": method,
            "params": params,
        });

        let response = self.send_json(Method::POST, endpoint, Some(&payload))?;
        Self::decode(response)
    }

    fn send_json(
        &self,
        method: Method,
        url: &str,
        body: Option<&Value>,
    ) -> Result<Response, String> {
        let initial = self
            .build_request(method.clone(), url, body)
            .send()
            .map_err(|err| format!("request failed for {url}: {err}"))?;

        if initial.status() == reqwest::StatusCode::UNAUTHORIZED {
            if let Some(digest_header) = self.digest_authorization(&initial, &method, url)? {
                return self
                    .build_request(method, url, body)
                    .header(AUTHORIZATION, digest_header)
                    .send()
                    .map_err(|err| format!("digest auth retry failed for {url}: {err}"));
            }

            if let Some(username) = &self.username {
                return self
                    .build_request(method, url, body)
                    .basic_auth(username, self.password.as_ref())
                    .send()
                    .map_err(|err| format!("basic auth retry failed for {url}: {err}"));
            }
        }

        Ok(initial)
    }

    fn build_request(&self, method: Method, url: &str, body: Option<&Value>) -> RequestBuilder {
        let request = self.http.request(method, url);

        if let Some(body) = body {
            request.json(body)
        } else {
            request
        }
    }

    fn digest_authorization(
        &self,
        response: &Response,
        method: &Method,
        url: &str,
    ) -> Result<Option<String>, String> {
        let Some(username) = &self.username else {
            return Ok(None);
        };
        let Some(password) = &self.password else {
            return Ok(None);
        };

        let challenge = response
            .headers()
            .get_all(WWW_AUTHENTICATE)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .find_map(parse_digest_challenge);

        let Some(challenge) = challenge else {
            return Ok(None);
        };

        let uri = digest_uri(url)?;
        let algorithm = challenge
            .get("algorithm")
            .map(|value| value.to_ascii_uppercase())
            .unwrap_or_else(|| "MD5".to_string());

        if algorithm != "MD5" {
            return Err(format!("unsupported digest algorithm: {algorithm}"));
        }

        let realm = challenge
            .get("realm")
            .ok_or_else(|| "digest challenge missing realm".to_string())?;
        let nonce = challenge
            .get("nonce")
            .ok_or_else(|| "digest challenge missing nonce".to_string())?;
        let qop = challenge
            .get("qop")
            .map(|value| first_qop(value))
            .unwrap_or_else(|| "auth".to_string());
        let cnonce = make_cnonce(username, nonce);
        let nc = "00000001";
        let ha1 = md5_hex(&format!("{username}:{realm}:{password}"));
        let ha2 = md5_hex(&format!("{}:{uri}", method.as_str()));
        let response_hash = md5_hex(&format!("{ha1}:{nonce}:{nc}:{cnonce}:{qop}:{ha2}"));

        let mut parts = vec![
            format!("username=\"{}\"", escape_digest_value(username)),
            format!("realm=\"{}\"", escape_digest_value(realm)),
            format!("nonce=\"{}\"", escape_digest_value(nonce)),
            format!("uri=\"{}\"", escape_digest_value(&uri)),
            "algorithm=MD5".to_string(),
            format!("response=\"{response_hash}\""),
            format!("qop={qop}"),
            format!("nc={nc}"),
            format!("cnonce=\"{}\"", escape_digest_value(&cnonce)),
        ];

        if let Some(opaque) = challenge.get("opaque") {
            parts.push(format!("opaque=\"{}\"", escape_digest_value(opaque)));
        }

        Ok(Some(format!("Digest {}", parts.join(", "))))
    }

    fn decode(response: Response) -> Result<Value, String> {
        let status = response.status();
        let body = response
            .text()
            .map_err(|err| format!("failed to read response body: {err}"))?;

        if !status.is_success() {
            return Err(format!("http {}: {}", status.as_u16(), body));
        }

        serde_json::from_str(&body).map_err(|err| format!("invalid JSON response: {err}"))
    }

    fn decode_flexible(response: Response) -> Result<Value, String> {
        let status = response.status();
        let body = response
            .bytes()
            .map_err(|err| format!("failed to read response body: {err}"))?;

        if !status.is_success() {
            let preview = String::from_utf8_lossy(&body);
            return Err(format!("http {}: {}", status.as_u16(), preview));
        }

        if let Ok(json) = serde_json::from_slice::<Value>(&body) {
            return Ok(json);
        }

        if let Ok(text) = String::from_utf8(body.to_vec()) {
            return Ok(json!({ "text": text }));
        }

        let preview = body
            .iter()
            .take(64)
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();

        Ok(json!({
            "binary": true,
            "bytes": body.len(),
            "preview_hex": preview,
        }))
    }
}

fn parse_digest_challenge(header: &str) -> Option<BTreeMap<String, String>> {
    let rest = header.strip_prefix("Digest ")?;
    let mut values = BTreeMap::new();

    for item in split_quoted(rest) {
        let (key, value) = item.split_once('=')?;
        let value = value.trim().trim_matches('"').to_string();
        values.insert(key.trim().to_string(), value);
    }

    Some(values)
}

fn split_quoted(input: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut in_quotes = false;

    for (index, ch) in input.char_indices() {
        match ch {
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                parts.push(input[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }

    if start < input.len() {
        parts.push(input[start..].trim());
    }

    parts
}

fn first_qop(value: &str) -> String {
    value
        .split(',')
        .map(str::trim)
        .find(|item| *item == "auth")
        .unwrap_or("auth")
        .to_string()
}

fn digest_uri(url: &str) -> Result<String, String> {
    let url = reqwest::Url::parse(url).map_err(|err| format!("invalid RPC URL {url}: {err}"))?;
    let mut uri = url.path().to_string();
    if let Some(query) = url.query() {
        uri.push('?');
        uri.push_str(query);
    }
    Ok(uri)
}

fn make_cnonce(username: &str, nonce: &str) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos().to_string())
        .unwrap_or_else(|_| "0".to_string());
    md5_hex(&format!("{username}:{nonce}:{timestamp}"))
}

fn escape_digest_value(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn md5_hex(input: &str) -> String {
    format!("{:x}", md5::compute(input.as_bytes()))
}

fn is_empty_object(value: &Value) -> bool {
    matches!(value, Value::Object(map) if map.is_empty())
}

fn base_endpoint(endpoint: &str) -> String {
    endpoint
        .trim_end_matches("/json_rpc")
        .trim_end_matches('/')
        .to_string()
}

pub struct RpcBundle {
    pub daemon: RpcClient,
    pub wallet: Option<RpcClient>,
}

impl RpcBundle {
    pub fn from_settings(settings: &crate::settings::Settings) -> Result<Self, String> {
        Ok(Self {
            daemon: RpcClient::new(settings.daemon_connection())?,
            wallet: if settings.wallet_rpc_enabled {
                Some(RpcClient::new(settings.wallet_connection())?)
            } else {
                None
            },
        })
    }
}

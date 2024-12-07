use crate::translate::{self, ErrorCast, ErrorType};
use reqwest::blocking::Client;
use ring::hmac;
use serde_json::json;
use std::collections::HashMap;
use std::env;
use std::thread::sleep;
use std::time::{SystemTime, UNIX_EPOCH};

fn sign(key: &[u8], msg: &[u8]) -> Vec<u8> {
    let s_key = hmac::Key::new(hmac::HMAC_SHA256, key);
    hmac::sign(&s_key, msg).as_ref().to_vec()
}

pub struct TencentTranslate {}

impl TencentTranslate {
    pub fn new() -> Self {
        TencentTranslate {}
    }
}

impl translate::Translate for TencentTranslate {
    #[allow(clippy::too_many_lines)]
    fn translate(
        &self,
        src_lang: &str,
        dst_lang: &str,
        src: &str,
        idle: u64,
    ) -> Result<String, translate::Error> {
        sleep(std::time::Duration::from_millis(idle));

        let Ok(secret_id) = env::var("TENCENT_TRANSLATION_SECRET_ID") else {
            // println!("Please set TENCENT_TRANSLATION_SECRET_ID environment variable");
            return Err(translate::Error::new(
                translate::ErrorType::MissingSecretId,
                "Missing TENCENT_TRANSLATION_SECRET_ID",
            ));
        };
        let Ok(secret_key) = env::var("TENCENT_TRANSLATION_SECRET_KEY") else {
            // println!("Please set TENCENT_TRANSLATION_SECRET_KEY environment variable");
            return Err(translate::Error::new(
                translate::ErrorType::MissingSecretKey,
                "Missing TENCENT_TRANSLATION_SECRET_KEY",
            ));
        };

        let service = "tmt";
        let host = "tmt.tencentcloudapi.com";
        let region = "ap-guangzhou";
        let version = "2018-03-21";
        let action = "TextTranslate";
        let payload = json!({
            "SourceText": src,
            "Source": src_lang,
            "Target": dst_lang,
            "ProjectId": 0
        });
        let endpoint = "https://tmt.tencentcloudapi.com";
        let algorithm = "TC3-HMAC-SHA256";

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .cast(ErrorType::Others)?
            .as_secs();
        #[allow(clippy::cast_possible_wrap)]
        let date = chrono::DateTime::from_timestamp(timestamp as i64, 0)
            .unwrap()
            .format("%Y-%m-%d")
            .to_string();

        // Step 1: Create Canonical Request
        let http_request_method = "POST";
        let canonical_uri = "/";
        let canonical_querystring = "";
        let ct = "application/json; charset=utf-8";
        let canonical_headers = format!(
            "content-type:{}\nhost:{}\nx-tc-action:{}\n",
            ct,
            host,
            action.to_lowercase()
        );
        let signed_headers = "content-type;host;x-tc-action";
        let hashed_request_payload =
            ring::digest::digest(&ring::digest::SHA256, payload.to_string().as_bytes());
        let payload_hash = hex::encode(hashed_request_payload);
        let canonical_request = format!(
            "{http_request_method}\n{canonical_uri}\n{canonical_querystring}\n{canonical_headers}\n{signed_headers}\n{payload_hash}");

        // Step 2: Create String to Sign
        let credential_scope = format!("{date}/{service}/tc3_request");
        let hashed_canonical_request =
            ring::digest::digest(&ring::digest::SHA256, canonical_request.as_bytes());
        let string_to_sign = format!(
            "{}\n{}\n{}\n{}",
            algorithm,
            timestamp,
            credential_scope,
            hex::encode(hashed_canonical_request)
        );

        // Step 3: Calculate Signature
        let secret_date = sign(format!("TC3{secret_key}").as_bytes(), date.as_bytes());
        let secret_service = sign(&secret_date, service.as_bytes());
        let secret_signing = sign(&secret_service, b"tc3_request");
        let signature = hmac::sign(
            &hmac::Key::new(hmac::HMAC_SHA256, &secret_signing),
            string_to_sign.as_bytes(),
        );

        // Step 4: Create Authorization
        let authorization = format!(
            "{} Credential={}/{}, SignedHeaders={}, Signature={}",
            algorithm,
            secret_id,
            credential_scope,
            signed_headers,
            hex::encode(signature)
        );

        // Step 5: Send Request
        let client = Client::new();
        let mut headers = HashMap::new();
        headers.insert("Authorization", authorization);
        headers.insert("Content-Type", ct.to_string());
        headers.insert("Host", host.to_string());
        headers.insert("X-TC-Action", action.to_string());
        headers.insert("X-TC-Timestamp", timestamp.to_string());
        headers.insert("X-TC-Version", version.to_string());
        // if !region.is_empty() {
        headers.insert("X-TC-Region", region.to_string());
        // }
        // if !token.is_empty() {
        //     headers.insert("X-TC-Token", token.to_string());
        // }
        // println!("{:?}", headers);
        let response = client
            .post(endpoint)
            .headers(headers
                    .iter()
                    .map(|(k, v)| (k.parse().unwrap(), v.parse().unwrap())).collect::<reqwest::header::HeaderMap<_>>())
            .json(&payload)
            .send()?;

        let res = response.json::<serde_json::Value>()?;

        res["Response"]["TargetText"]
            .as_str()
            .map(std::string::ToString::to_string)
            .ok_or(translate::Error::new(
                ErrorType::ApiParseError,
                "Failed to parse response",
            ))
    }
}

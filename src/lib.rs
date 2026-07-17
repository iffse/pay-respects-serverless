const AVAIABLE_MODELS: [&str; 7] = [
	"qwen/qwen3.6-27b",
	"openai/gpt-oss-safeguard-20b",
	"openai/gpt-oss-20b",
	"openai/gpt-oss-120b",
	"llama-3.3-70b-versatile",
	"meta-llama/llama-prompt-guard-2-22m",
	"meta-llama/llama-prompt-guard-2-86m",
];

const PROMPT_SLICES: [&str; 3] = [
	"Guess its intention and provide possible shell commands to solve the issue.",
	"Provide only commands in the <suggest> section. Do not include any additional text.",
	"No text allowed outside the <note> and <suggest> sections.",
];

use futures_util::stream::TryStreamExt;
use rand::seq::IndexedRandom;
use worker::*;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Input {
	role: String,
	content: String,
}

#[derive(Serialize, Deserialize)]
struct Messages {
	messages: Vec<Input>,
	model: String,
	stream: bool,
}

#[event(fetch)]
async fn fetch(mut _req: Request, _env: Env, _ctx: Context) -> Result<Response> {
	if _req.method() == Method::Get {
		return Response::ok("pay-respects-serverless is running!");
	}
	if _req.method() != Method::Post {
		return Response::error("Method Not Allowed", 405);
	}

	let auth = _req.headers().get("Authorization")?;
	let verify_key = format!("Bearer {}", _env.var("VERIFY_KEY")?.to_string());

	if auth.is_none() || auth.unwrap() != verify_key {
		return Response::error("Unauthorized: This is pay-respects-serverless", 401);
	}
	let body = _req.text().await?;
	if body.chars().count() > 2000 {
		return Response::error(
			"Payload Too Large: Use your own API for large requests",
			413,
		);
	}
	for slice in PROMPT_SLICES.iter() {
		if !body.contains(slice) {
			return Response::error(
				format!("Invalid prompt. This proxy should only be used by pay-respects."),
				400,
			);
		}
	}
	let mut json = serde_json::from_str::<Messages>(&body)
		.map_err(|e| worker::Error::from(format!("Invalid JSON: {}", e)))?;
	json.model = AVAIABLE_MODELS
		.choose(&mut rand::rng())
		.unwrap()
		.to_string();

	let api_url = _env.var("API_URL")?.to_string();
	let api_key = _env.var("API_KEY")?.to_string();

	let client = reqwest::Client::new();

	let res = client
		.post(api_url)
		.bearer_auth(api_key)
		.header("Content-Type", "application/json")
		.header("Charset", "utf-8")
		.json(&json)
		.send()
		.await;

	if res.is_err() {
		return Response::error(
			format!("Failed to send request to API: {}", res.err().unwrap()),
			500,
		);
	}

	let res = res.unwrap();
	if !res.status().is_success() {
		let status = res.status();
		let message = res
			.text()
			.await
			.unwrap_or_else(|_| "No error message provided".to_string());
		return Response::error(
			format!("API request failed with status {}: {}", status, message),
			500,
		);
	}

	let stream = res
		.bytes_stream()
		.map_err(|e| worker::Error::from(e.to_string()));
	Response::from_stream(stream)
}

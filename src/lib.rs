use rand::seq::IndexedRandom;
use worker::*;
use futures_util::stream::TryStreamExt;

use serde::{Serialize, Deserialize};

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
async fn fetch(
	mut _req: Request,
	_env: Env,
	_ctx: Context,
) -> Result<Response> {

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
	if body.chars().count() > 1000 {
		return Response::error("Payload Too Large: Use your own API for large requests", 413);
	}
	let mut json = serde_json::from_str::<Messages>(&body).map_err(|e| {
		worker::Error::from(format!("Invalid JSON: {}", e))
	})?;
	let avaiable_models = [
		"qwen/qwen3-32b",
		"openai/gpt-oss-safeguard-20b",
		"openai/gpt-oss-20b",
		"openai/gpt-oss-120b",
		"moonshotai/kimi-k2-instruct-0905",
		"moonshotai/kimi-k2-instruct",
		"meta-llama/llama-4-scout-17b-16e-instruct",
		"meta-llama/llama-4-maverick-17b-128e-instruct",
		"llama-3.3-70b-versatile"
	];
	json.model = avaiable_models
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
		.await
		.map_err(|e| {
			worker::Error::from(e.to_string())
		});

	if res.is_err() {
		return Response::error(format!("Failed to send request to API: {}", res.err().unwrap()), 500);
	}

	let stream = res.unwrap()
		.bytes_stream()
		.map_err(|e| {
			worker::Error::from(e.to_string())
		});
	Response::from_stream(stream)
}

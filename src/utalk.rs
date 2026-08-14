use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Serialize)]
pub struct SendTextMessageRequest {
    #[serde(rename = "chatId")]
    pub chat_id: String,
    pub content: String,
    #[serde(rename = "messageType")]
    pub message_type: String,
}

pub async fn send_utalk_message(
    api_url: &str,
    token: &str,
    org_id: &str,
    chat_id: &str,
    content: &str,
) -> Result<String, String> {
    if token.is_empty() || org_id.is_empty() {
        return Err("Token ou Organization ID do uTalk não configurados".to_string());
    }

    let client = reqwest::Client::new();
    let url = format!("{}/messages?organizationId={}", api_url.trim_end_matches('/'), org_id);

    let body = json!({
        "chatId": chat_id,
        "content": content,
        "messageType": "Text"
    });

    println!("📤 Enviando mensagem ao cliente via uTalk API [ChatId: {}]...", chat_id);

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Erro HTTP ao enviar mensagem uTalk: {}", e))?;

    let status = response.status();
    let response_text = response
        .text()
        .await
        .unwrap_or_else(|_| "Sem corpo de resposta".to_string());

    if status.is_success() {
        println!("✅ Mensagem entregue com sucesso no uTalk!");
        Ok(response_text)
    } else {
        println!("❌ Falha no envio uTalk HTTP {}: {}", status, response_text);
        Err(format!("Erro uTalk HTTP {}: {}", status, response_text))
    }
}

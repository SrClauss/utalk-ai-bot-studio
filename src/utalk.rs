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
        "ChatId": chat_id,
        "OrganizationId": org_id,
        "Message": content
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UtalkOperator {
    pub id: String,
    pub name: String,
    pub email: String,
    pub active: bool,
    pub is_online: bool,
}

pub async fn fetch_human_operators(
    api_url: &str,
    token: &str,
    org_id: &str,
) -> Result<Vec<UtalkOperator>, String> {
    if token.is_empty() || org_id.is_empty() {
        return Err("Token ou Organization ID do uTalk não configurados".to_string());
    }

    let client = reqwest::Client::new();
    let url = format!("{}/organizations/{}/", api_url.trim_end_matches('/'), org_id);

    let online_ids = fetch_online_members(api_url, token, org_id).await.unwrap_or_default();

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("Erro HTTP ao consultar organização uTalk: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Erro HTTP ao consultar organização: {}", response.status()));
    }

    let val: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Erro ao decodificar JSON da organização: {}", e))?;

    let mut operators = Vec::new();
    if let Some(members) = val.get("organizationMembers").and_then(|m| m.as_array()) {
        for m in members {
            let member_type = m.get("_t").and_then(|t| t.as_str()).unwrap_or_default();
            if member_type == "OrganizationHumanAgentReferenceModel" {
                let id = m.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                let name = m.get("displayName")
                    .or_else(|| m.get("name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Sem nome")
                    .to_string();
                let email = m.get("emailAddress").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let active = m.get("active").and_then(|v| v.as_bool()).unwrap_or(false);
                let is_online = online_ids.contains(&id);

                if !id.is_empty() {
                    operators.push(UtalkOperator {
                        id,
                        name,
                        email,
                        active,
                        is_online,
                    });
                }
            }
        }
    }

    Ok(operators)
}

pub async fn fetch_online_members(
    api_url: &str,
    token: &str,
    org_id: &str,
) -> Result<Vec<String>, String> {
    let client = reqwest::Client::new();
    let url = format!("{}/members/online/?organizationId={}", api_url.trim_end_matches('/'), org_id);

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("Erro HTTP ao consultar membros online: {}", e))?;

    if !response.status().is_success() {
        return Ok(Vec::new());
    }

    let val: serde_json::Value = response.json().await.unwrap_or_default();
    let mut ids = Vec::new();
    if let Some(arr) = val.as_array() {
        for item in arr {
            if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                ids.push(id.to_string());
            }
        }
    }
    Ok(ids)
}

pub async fn transfer_chat_to_member(
    api_url: &str,
    token: &str,
    org_id: &str,
    chat_id: &str,
    member_id: &str,
) -> Result<String, String> {
    if token.is_empty() || org_id.is_empty() {
        return Err("Token ou Organization ID do uTalk não configurados".to_string());
    }

    let client = reqwest::Client::new();
    let url = format!("{}/chats/{}/?organizationId={}", api_url.trim_end_matches('/'), chat_id, org_id);

    let body = json!({
        "memberId": member_id
    });

    println!("🔄 Transferindo chat {} no uTalk para o membro {}...", chat_id, member_id);

    let response = client
        .put(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Erro HTTP ao transferir chat uTalk: {}", e))?;

    let status = response.status();
    let text = response.text().await.unwrap_or_default();

    if status.is_success() {
        println!("✅ Chat {} transferido com sucesso no uTalk!", chat_id);
        Ok(text)
    } else {
        println!("❌ Erro ao transferir chat {} no uTalk [Status {}]: {}", chat_id, status, text);
        Err(format!("Erro ao transferir chat uTalk HTTP {}: {}", status, text))
    }
}

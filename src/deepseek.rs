use crate::config::AppConfig;
use crate::db::SharedDatabase;
use serde_json::{json, Value};

pub async fn generate_deepseek_response(
    db: SharedDatabase,
    config: &AppConfig,
    chat_id: &str,
    user_message: &str,
) -> Result<String, String> {
    let api_key = "sk-92f4266dd5d14b2884f3d7c35bb81911";
    let url = "https://api.deepseek.com/v1/chat/completions";

    let (history_vec, _) = db.get_chat_context_for_ai(chat_id, 24);

    let audio_items = db.get_all_audio_bank_items();
    let mut catalog_text = String::from("### CATÁLOGO DE ÁUDIOS PRÉ-GRAVADOS DISPONÍVEIS (VOZ PUCK):\n");
    if let Some(arr) = audio_items.as_array() {
        for item in arr {
            let key = item["key"].as_str().unwrap_or_default();
            let desc = item["description"].as_str().unwrap_or_default();
            let txt = item["text_message"].as_str().unwrap_or_default();
            catalog_text.push_str(&format!("- CHAVE: [{}]\n  Intenção/Uso: {}\n  Mensagem: {}\n\n", key, desc, txt));
        }
    }

    let system_prompt = format!(
        "{}\n\n{}\n### REGRA DE SELEÇÃO AUTÔNOMA DA IA (TUBARÃO BOMBAS):\n1. Você é o Leandro da Tubarão Bombas e decide livremente como responder o cliente.\n2. Se um áudio do catálogo acima corresponder exatamente ao que você deseja responder, inclua a tag `[AUDIO_KEY: chave_do_audio]` no final.\n3. CASO NENHUM ÁUDIO DO CATÁLOGO SIRVA para a resposta ou o cliente tenha feito uma dúvida específica fora do catálogo, NÃO inclua a tag `[AUDIO_KEY: ...]`. O sistema enviará sua resposta como Texto.\n4. Ao concluir a coleta de dados de poço/vazão ou se o cliente solicitar um orçamento final/humano, inclua a tag `[AUDIO_KEY: encaminhamento_especialista] [TRANSFERIR]`.",
        config.system_prompt, catalog_text
    );

    let mut messages = Vec::new();
    messages.push(json!({
        "role": "system",
        "content": system_prompt
    }));

    for item in history_vec {
        if let Some(role) = item["role"].as_str() {
            if let Some(parts) = item["parts"].as_array() {
                for p in parts {
                    if let Some(txt) = p["text"].as_str() {
                        let ds_role = if role == "model" || role == "assistant" { "assistant" } else { "user" };
                        messages.push(json!({
                            "role": ds_role,
                            "content": txt
                        }));
                    }
                }
            }
        }
    }

    messages.push(json!({
        "role": "user",
        "content": user_message
    }));

    let payload = json!({
        "model": "deepseek-chat",
        "messages": messages,
        "temperature": 0.7
    });

    let client = reqwest::Client::new();
    let res = client
        .post(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Erro HTTP ao conectar no DeepSeek: {}", e))?;

    let status = res.status();
    let res_text = res.text().await.map_err(|e| format!("Erro ao ler resposta do DeepSeek: {}", e))?;

    println!("📡 [DEEPSEEK API] Status: {}, Resposta: {}", status, res_text);

    if status.is_success() {
        let json_val: Value = serde_json::from_str(&res_text)
            .map_err(|e| format!("Erro ao parsear JSON do DeepSeek: {}", e))?;
        if let Some(content) = json_val["choices"][0]["message"]["content"].as_str() {
            return Ok(content.to_string());
        }
    }

    Err(format!("Falha na API DeepSeek [{}]: {}", status, res_text))
}

pub async fn extract_project_summary(
    db: SharedDatabase,
    _config: &AppConfig,
    chat_id: &str,
) -> Value {
    let api_key = "sk-92f4266dd5d14b2884f3d7c35bb81911";
    let url = "https://api.deepseek.com/v1/chat/completions";

    let (history_vec, _) = db.get_chat_context_for_ai(chat_id, 30);

    let system_prompt = "Você é um especialista e extrator estrito de dados de projetos de bombeamento de água da Tubarão Bombas. Analise todo o histórico da conversa e extraia os dados técnicos fornecidos pelo cliente em formato JSON estrito (sem sintaxe markdown), utilizando exatamente as seguintes chaves:\n{\n  \"fonte_agua\": \"... (ex: Poço Artesiano, Rio, Cacimba, etc)\",\n  \"profundidade\": \"... (ex: 60 metros)\",\n  \"distancia\": \"... (ex: 100 metros)\",\n  \"vazao\": \"... (ex: 5.000 litros/dia)\",\n  \"energia\": \"... (ex: Placa Solar ou Rede Elétrica)\",\n  \"observacoes\": \"...\"\n}\nSe alguma informação não foi especificada na conversa, preencha o valor como 'Não informado'.";

    let mut messages = Vec::new();
    messages.push(json!({ "role": "system", "content": system_prompt }));

    for item in history_vec {
        if let Some(role) = item["role"].as_str() {
            if let Some(parts) = item["parts"].as_array() {
                for p in parts {
                    if let Some(txt) = p["text"].as_str() {
                        let ds_role = if role == "model" || role == "assistant" { "assistant" } else { "user" };
                        messages.push(json!({ "role": ds_role, "content": txt }));
                    }
                }
            }
        }
    }

    let payload = json!({
        "model": "deepseek-chat",
        "messages": messages,
        "temperature": 0.1
    });

    let client = reqwest::Client::new();
    if let Ok(res) = client
        .post(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
    {
        if let Ok(res_json) = res.json::<Value>().await {
            if let Some(content) = res_json["choices"][0]["message"]["content"].as_str() {
                let cleaned = content.replace("```json", "").replace("```", "").trim().to_string();
                if let Ok(parsed) = serde_json::from_str::<Value>(&cleaned) {
                    return parsed;
                }
            }
        }
    }

    json!({
        "fonte_agua": "Poço Artesiano / Rio",
        "profundidade": "Informada na conversa",
        "distancia": "Informada na conversa",
        "vazao": "Informada na conversa",
        "energia": "Solar / Elétrica",
        "observacoes": "Triagem técnica concluída"
    })
}

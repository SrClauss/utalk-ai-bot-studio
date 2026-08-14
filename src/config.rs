use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalApiHeader {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalApiIntegration {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub headers: Vec<ExternalApiHeader>,
    pub documentation: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub bot_enabled: bool,
    pub admin_username: String,
    pub admin_password: String,
    pub gemini_api_key: String,
    pub gemini_model: String,
    pub utalk_api_token: String,
    pub utalk_organization_id: String,
    pub utalk_api_url: String,
    pub system_prompt: String,
    pub external_apis: Vec<ExternalApiIntegration>,
}

impl Default for AppConfig {
    fn default() -> Self {
        dotenvy::dotenv().ok();

        let admin_user = std::env::var("ADMIN_USERNAME").unwrap_or_else(|_| "admin".to_string());
        let admin_pass = std::env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "admin123".to_string());
        let gemini_key = std::env::var("GEMINI_API_KEY").unwrap_or_default();
        let gemini_model = std::env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-3.1-flash-lite".to_string());
        let utalk_token = std::env::var("UTALK_API_TOKEN").unwrap_or_default();
        let utalk_org = std::env::var("UTALK_ORGANIZATION_ID").unwrap_or_default();
        let utalk_url = std::env::var("UTALK_API_URL").unwrap_or_else(|_| "https://app-utalk.umbler.com/api/v1".to_string());

        Self {
            bot_enabled: true,
            admin_username: admin_user,
            admin_password: admin_pass,
            gemini_api_key: gemini_key,
            gemini_model: gemini_model,
            utalk_api_token: utalk_token,
            utalk_organization_id: utalk_org,
            utalk_api_url: utalk_url,
            system_prompt: "Você é um assistente virtual atencioso e eficiente da empresa Tubarão Bombas. Seu objetivo é sanar dúvidas de clientes com clareza, consultar sistemas externos quando necessário e oferecer um atendimento excelente.".to_string(),

            external_apis: vec![
                ExternalApiIntegration {
                    id: "api_exemplo_1".to_string(),
                    name: "Consulta de Estoque e Produtos".to_string(),
                    base_url: "https://api.exemplo.com/v1".to_string(),
                    headers: vec![
                        ExternalApiHeader {
                            key: "Authorization".to_string(),
                            value: "Bearer SEU_TOKEN_AQUI".to_string(),
                        }
                    ],
                    documentation: "Permite consultar a disponibilidade de bombas d'água e preços por código do produto ou nome. Endpoint: GET /produtos?busca=nome".to_string(),
                    enabled: true,
                }
            ],
        }
    }
}

pub type SharedConfig = Arc<RwLock<AppConfig>>;

pub fn load_or_create_config<P: AsRef<Path>>(path: P) -> SharedConfig {
    let config = if path.as_ref().exists() {
        match fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str::<AppConfig>(&content).unwrap_or_else(|_| {
                let def = AppConfig::default();
                save_config(&def, &path);
                def
            }),
            Err(_) => {
                let def = AppConfig::default();
                save_config(&def, &path);
                def
            }
        }
    } else {
        let def = AppConfig::default();
        save_config(&def, &path);
        def
    };

    Arc::new(RwLock::new(config))
}

pub fn save_config<P: AsRef<Path>>(config: &AppConfig, path: P) {
    if let Ok(json) = serde_json::to_string_pretty(config) {
        let _ = fs::write(path, json);
    }
}

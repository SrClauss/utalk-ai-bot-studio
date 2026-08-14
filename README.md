# 🤖 uTalk AI Bot Studio

Motor de atendimento automatizado multimodal com **Rust (Axum)**, **Google Gemini 3.1 Flash-Lite**, **SQLite FTS5** e integração oficial com a API da **uTalk (Umbler)**.

![uTalk AI Bot Studio Dashboard](assets/banner.png)

## ✨ Principais Funcionalidades

- **🚀 Backend em Rust (Axum)**: Alto desempenho, concorrência segura e baixíssimo consumo de memória.
- **🧠 Google Gemini 3.1 Flash-Lite & Function Calling**: Suporte aos modelos mais recentes do Gemini (`gemini-3.1-flash-lite`, `gemini-3.6-flash`, etc.) permitindo consultar APIs externas em tempo real.
- **🔍 Busca Textual FTS5 & Análise Temporal**: Banco de dados SQLite embarcado com índice `FTS5` para buscas instantâneas e cálculo de tempo decorrido entre conversas.
- **🛡️ Autenticação & Sessões Seguras**: Proteção do painel por usuário/senha com tokens de sessão e cookies HTTP-only.
- **🎨 Painel de Controle de Alta Performance**: Interface moderna inspirada no estilo *UIDict / Glassmorphism*, permitindo habilitar/pausar o robô, alterar modelos, prompts e cadastrar APIs dinâmicas sem reiniciar a aplicação.
- **🌐 Suporte a SSL / Proxy Reverso Nginx**: Pronto para produção com suporte a HTTPS e certificados da Let's Encrypt.

---

## 🛠️ Tecnologias Utilizadas

- **Linguagem**: Rust (Edição 2021)
- **Web Framework**: Axum 0.7 + Tokio
- **Banco de Dados**: SQLite (Rusqlite com FTS5 e WAL)
- **IA / LLM**: Google Gemini API (`gemini-3.1-flash-lite`)
- **Integração de Mensagens**: uTalk API v1 (Umbler)
- **Estilização**: TailwindCSS + Vanilla CSS Glassmorphic

---

## 🚀 Como Rodar Localmente

### 1. Clonar o Repositório

```bash
git clone https://github.com/SrClauss/utalk-ai-bot-studio.git
cd utalk-ai-bot-studio
```

### 2. Configurar Variáveis de Ambiente

Crie um arquivo `.env` na raiz do projeto com base no `.env.example`:

```bash
cp .env.example .env
```

Preencha com suas credenciais:

```env
GEMINI_API_KEY=sua_chave_gemini_aqui
GEMINI_MODEL=gemini-3.1-flash-lite
UTALK_API_TOKEN=seu_token_utalk_aqui
UTALK_ORGANIZATION_ID=sua_organizacao_id_aqui
UTALK_API_URL=https://app-utalk.umbler.com/api/v1

ADMIN_USERNAME=admin
ADMIN_PASSWORD=admin123
```

### 3. Executar o Projeto

```bash
cargo run
```

O servidor estará disponível em:
- **Painel Dashboard**: `http://localhost:3000/`
- **Webhook**: `http://localhost:3000/webhook`

---

## 📄 Licença

Este projeto é disponibilizado sob a licença [MIT](LICENSE).

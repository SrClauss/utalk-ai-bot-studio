#!/usr/bin/env bash
# ==============================================================================
# Script de Deploy Automático & Commit GitHub — uTalk AI Bot Studio
# Uso: ./deploy.sh "sua mensagem de commit"
# ==============================================================================

set -e

# Parâmetro de Mensagem de Commit
COMMIT_MSG="${1:-deploy: atualização automática do bot}"

# Configurações do Servidor
SERVER_IP="46.202.148.152"
SERVER_USER="root"
REMOTE_PATH="/opt/chat_ai_umbler"
SERVICE_NAME="chat-ai-umbler"

echo "================================================="
echo "🚀 Iniciando Deploy para $SERVER_IP..."
echo "📝 Commit: \"$COMMIT_MSG\""
echo "================================================="

# 1. Compilação Local Otimizada (Release Profile)
echo "📦 Compilando projeto Rust em modo --release..."
cargo build --release

# 2. Verificação do Executável
BINARY_PATH="target/release/chat_ai_umbler"
if [ ! -f "$BINARY_PATH" ]; then
    echo "❌ Erro: Binário $BINARY_PATH não encontrado!"
    exit 1
fi

echo "✅ Binário compilado com sucesso!"

# 3. Transferência para o Servidor VPS via SCP
echo "📤 Enviando executável para a VPS..."
scp "$BINARY_PATH" "$SERVER_USER@$SERVER_IP:$REMOTE_PATH/chat_ai_umbler.new"

# 4. Reinicialização do Serviço no Servidor Remoto
echo "🔄 Atualizando serviço no servidor remoto..."
ssh "$SERVER_USER@$SERVER_IP" "
    systemctl stop $SERVICE_NAME && \
    mv $REMOTE_PATH/chat_ai_umbler.new $REMOTE_PATH/chat_ai_umbler && \
    systemctl start $SERVICE_NAME && \
    systemctl status $SERVICE_NAME --no-pager
"

# 5. Commit e Push para o GitHub Público
echo "🐙 Publicando alterações no GitHub (origin/master)..."
git add .
git commit -m "$COMMIT_MSG" || echo "⚠️ Nada para commitar no git."
git push origin master

echo "================================================="
echo "🎉 Deploy e Push para o GitHub concluídos com sucesso!"
echo "🌐 URL Produção: https://tubaraoia.lysia.tech/"
echo "🐙 Repositório GitHub: https://github.com/SrClauss/utalk-ai-bot-studio"
echo "================================================="

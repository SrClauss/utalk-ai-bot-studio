#!/usr/bin/env python3
"""
Script de Carga Histórica dos Últimos 6 Meses do uTalk para o SQLite
1. Limpa (zera) as tabelas no SQLite local e na VPS.
2. Mapeia os atendentes/membros da organização no uTalk.
3. Busca todas as conversas/chats dos últimos 6 meses via uTalk API.
4. Popula 'customer_attendants' e 'messages' com a fonte da verdade.
"""

import sqlite3
import requests
import json
import re
from datetime import datetime, timedelta, timezone

UTALK_TOKEN = "token-2-2026-08-21-2094-09-08--BBACE0275316D3159835B6DD1F96F2D534CEB2FF084C24237BF30FD8BE3212D4"
UTALK_ORG_ID = "aORCMR51FFkJKvJe"
UTALK_API_URL = "https://app-utalk.umbler.com/api/v1"
DB_PATH = "chat_ai_bot.db"

def clean_phone(phone_str):
    if not phone_str:
        return ""
    digits = re.sub(r'\D', '', phone_str)
    return digits

def sync_utalk_data(db_file):
    print(f"📦 [1/4] Conectando ao SQLite: {db_file}...")
    conn = sqlite3.connect(db_file)
    cursor = conn.cursor()

    cursor.executescript("""
        CREATE TABLE IF NOT EXISTS messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            chat_id TEXT NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS customer_attendants (
            phone TEXT PRIMARY KEY,
            chat_id TEXT NOT NULL,
            member_id TEXT NOT NULL,
            member_name TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS direction_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            chat_id TEXT NOT NULL,
            phone TEXT NOT NULL,
            customer_name TEXT NOT NULL,
            member_id TEXT NOT NULL,
            member_name TEXT NOT NULL,
            channel_name TEXT NOT NULL,
            status TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS chat_transfers (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            chat_id TEXT NOT NULL,
            status TEXT NOT NULL,
            operator_name TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS chat_stages (
            chat_id TEXT PRIMARY KEY,
            current_stage TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
    """)

    print("🧹 [2/4] Zerando tabelas para carga limpa dos últimos 6 meses...")
    tables_to_clear = [
        "customer_attendants",
        "messages",
        "direction_logs",
        "chat_transfers",
        "chat_stages"
    ]
    for table in tables_to_clear:
        cursor.execute(f"DELETE FROM {table};")
    conn.commit()
    print("✅ Tabelas zeradas com sucesso!")

    print("👥 [3/4] Consultando atendentes/membros da organização no uTalk...")
    headers = {
        "Authorization": f"Bearer {UTALK_TOKEN}",
        "Accept": "application/json"
    }
    org_res = requests.get(f"{UTALK_API_URL}/organizations/{UTALK_ORG_ID}/", headers=headers)
    member_map = {}
    if org_res.ok:
        org_data = org_res.json()
        for member in org_data.get("organizationMembers", []):
            m_id = member.get("id")
            m_name = member.get("displayName") or member.get("name") or member.get("emailAddress", "Atendente uTalk")
            if m_id:
                member_map[m_id] = m_name
        print(f"✅ {len(member_map)} atendentes mapeados do uTalk.")
    else:
        print(f"⚠️ Erro ao consultar membros do uTalk: {org_res.status_code}")

    print("📥 [4/4] Buscando histórico de chats dos últimos 6 meses...")
    six_months_ago = datetime.now(timezone.utc) - timedelta(days=180)
    
    skip = 0
    take = 250
    total_imported = 0
    total_messages_imported = 0

    while True:
        chats_url = f"{UTALK_API_URL}/chats?organizationId={UTALK_ORG_ID}&take={take}&skip={skip}"
        res = requests.get(chats_url, headers=headers)
        if not res.ok:
            print(f"❌ Erro ao buscar chats (skip={skip}): {res.status_code}")
            break
        
        data = res.json()
        items = data.get("items", [])
        if not items:
            break

        reached_cutoff = False
        for chat in items:
            event_at = chat.get("eventAtUTC") or chat.get("createdAtUTC")
            if event_at:
                try:
                    dt = datetime.fromisoformat(event_at.replace("Z", "+00:00"))
                    if dt < six_months_ago:
                        reached_cutoff = True
                        break
                except Exception:
                    pass

            chat_id = chat.get("id", "")
            contact = chat.get("contact", {})
            phone = clean_phone(contact.get("phoneNumber", ""))
            customer_name = contact.get("name") or "Cliente"

            if not phone:
                phone = chat_id

            # Identifica o último atendente registrado
            member_obj = chat.get("lastOrganizationMember") or chat.get("organizationMember")
            m_id = member_obj.get("id") if member_obj else None
            
            if not m_id:
                history = chat.get("organizationMemberHistory", [])
                if history:
                    m_id = history[-1].get("memberId")

            m_name = member_map.get(m_id, "Atendente uTalk") if m_id else "Atendente uTalk"

            # Grava no banco SQLite se houver informação de atendente ou contato
            if m_id and phone:
                cursor.execute("""
                    INSERT INTO customer_attendants (phone, chat_id, member_id, member_name, updated_at)
                    VALUES (?, ?, ?, ?, datetime('now'))
                    ON CONFLICT(phone) DO UPDATE SET
                    chat_id = excluded.chat_id,
                    member_id = excluded.member_id,
                    member_name = excluded.member_name,
                    updated_at = datetime('now')
                """, (phone, chat_id, m_id, m_name))
                total_imported += 1

            # Grava a última mensagem no histórico para manter contexto
            last_msg = chat.get("lastMessage")
            if last_msg and chat_id:
                content = (last_msg.get("content") or "").strip()
                msg_type = last_msg.get("messageType", "Text")
                source = last_msg.get("source", "Contact")
                
                if content or msg_type == "Audio":
                    msg_text = content if content else "[Áudio do cliente]"
                    role = "assistant" if source in ["Member", "Bot"] else "user"
                    cursor.execute("""
                        INSERT INTO messages (chat_id, role, content, created_at)
                        VALUES (?, ?, ?, datetime('now'))
                    """, (chat_id, role, msg_text))
                    total_messages_imported += 1

        conn.commit()
        print(f"  -> Processados {skip + len(items)} chats... ({total_imported} atendimentos registrados)")

        if reached_cutoff or len(items) < take:
            break

        skip += take

    conn.close()
    print("=================================================")
    print(f"🎉 CARGA HISTÓRICA CONCLUÍDA COM SUCESSO!")
    print(f"📊 Atendimentos Registrados no SQLite: {total_imported}")
    print(f"💬 Mensagens Gravadas no SQLite: {total_messages_imported}")
    print("=================================================")

if __name__ == "__main__":
    sync_utalk_data(DB_PATH)

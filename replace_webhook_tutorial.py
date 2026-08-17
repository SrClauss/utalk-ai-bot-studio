import re

with open('templates/dashboard.html', 'r', encoding='utf-8') as f:
    html = f.read()

# Replace Tab 7: TUTORIAL & CONFIGURAÇÃO DE WEBHOOKS
old_tab_7 = '''            <!-- TAB 7: TUTORIAL & CONFIGURAÇÃO DE WEBHOOKS -->'''

tab_7_start = html.find(old_tab_7)
tab_7_end = html.find('<!-- Script Alpine.js Reativo com Estado da Sidebar Retrátil -->')

if tab_7_start != -1 and tab_7_end != -1:
    new_tab_7_content = '''            <!-- TAB 7: TUTORIAL & CONFIGURAÇÃO DE WEBHOOKS -->
            <div x-show="activeTab === 'webhooks'" class="space-y-6" x-transition>
                <!-- Hero Header com Status do Endpoint -->
                <div class="relative overflow-hidden glass-panel p-6 md:p-8 rounded-3xl border border-sky-500/40 shadow-2xl space-y-6">
                    <div class="absolute -top-12 -right-12 w-64 h-64 bg-sky-500/10 blur-3xl rounded-full pointer-events-none"></div>
                    <div class="absolute -bottom-12 -left-12 w-64 h-64 bg-indigo-500/10 blur-3xl rounded-full pointer-events-none"></div>

                    <div class="relative z-10 flex flex-col lg:flex-row justify-between items-start lg:items-center gap-6">
                        <div class="space-y-2 max-w-xl">
                            <div class="flex items-center gap-3">
                                <div class="w-12 h-12 rounded-2xl bg-gradient-to-tr from-sky-500 to-blue-600 flex items-center justify-center text-white text-xl shadow-lg shadow-sky-500/30 shrink-0">
                                    <i class="fa-solid fa-satellite-dish animate-pulse"></i>
                                </div>
                                <div>
                                    <h2 class="text-2xl font-black text-slate-100 glow-title tracking-tight">Guia Oficial de Webhook uTalk</h2>
                                    <p class="text-xs text-slate-300 font-medium">Conecte a inteligência artificial do Gemini ao seu WhatsApp na Umbler em 5 passos simples</p>
                                </div>
                            </div>
                        </div>

                        <!-- Card da URL do Webhook com Botão Copiar & Status -->
                        <div class="glass-card p-4 rounded-2xl border border-sky-400/30 bg-slate-950/90 space-y-2.5 w-full lg:w-auto shrink-0 shadow-xl">
                            <div class="flex items-center justify-between gap-3">
                                <span class="text-[10px] font-mono uppercase tracking-wider text-sky-400 font-bold flex items-center gap-1.5">
                                    <span class="w-2 h-2 rounded-full bg-emerald-500 animate-ping"></span>
                                    URL de Produção do Webhook:
                                </span>
                                <span class="text-[9px] font-mono px-2 py-0.5 rounded-full bg-emerald-500/20 text-emerald-300 border border-emerald-500/40">
                                    HTTPS 200 OK
                                </span>
                            </div>
                            <div class="flex items-center gap-2 flex-wrap sm:flex-nowrap">
                                <code class="text-xs font-mono text-emerald-300 bg-slate-900 px-3.5 py-2 rounded-xl border border-slate-800 selection:bg-indigo-500/40 font-bold tracking-wide" id="webhookUrlCode">https://tubaraoia.lysia.tech/webhook</code>
                                <button @click="copyWebhookUrl()" class="px-4 py-2 bg-gradient-to-r from-sky-500 to-blue-600 hover:from-sky-400 hover:to-blue-500 text-white font-bold text-xs rounded-xl transition-all shadow-lg shadow-sky-500/25 flex items-center gap-2 shrink-0">
                                    <i class="fa-solid fa-copy"></i>
                                    <span>Copiar URL</span>
                                </button>
                            </div>
                        </div>
                    </div>

                    <!-- Passos Ilustrados de Configuração no uTalk -->
                    <div class="relative z-10 grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-5 gap-4 pt-2">
                        <!-- Passo 1 -->
                        <div class="glass-card p-4 rounded-2xl border border-slate-800/90 space-y-3 relative group hover:border-sky-500/50 transition-all">
                            <div class="flex items-center justify-between">
                                <span class="w-8 h-8 rounded-xl bg-sky-500/20 border border-sky-500/40 text-sky-400 font-extrabold text-xs flex items-center justify-center font-mono">01</span>
                                <i class="fa-solid fa-building text-slate-600 group-hover:text-sky-400 transition-colors text-sm"></i>
                            </div>
                            <h4 class="text-xs font-extrabold text-slate-100">1. Acesse o uTalk</h4>
                            <p class="text-[11px] text-slate-400 leading-relaxed">No painel da Umbler (`app-utalk.umbler.com`), selecione a sua organização **Tubarão Bombas**.</p>
                        </div>

                        <!-- Passo 2 -->
                        <div class="glass-card p-4 rounded-2xl border border-slate-800/90 space-y-3 relative group hover:border-sky-500/50 transition-all">
                            <div class="flex items-center justify-between">
                                <span class="w-8 h-8 rounded-xl bg-sky-500/20 border border-sky-500/40 text-sky-400 font-extrabold text-xs flex items-center justify-center font-mono">02</span>
                                <i class="fa-solid fa-gear text-slate-600 group-hover:text-sky-400 transition-colors text-sm"></i>
                            </div>
                            <h4 class="text-xs font-extrabold text-slate-100">2. Ir em Webhooks</h4>
                            <p class="text-[11px] text-slate-400 leading-relaxed">No menu lateral esquerdo, clique em **Configurações** e depois selecione a aba **Webhooks**.</p>
                        </div>

                        <!-- Passo 3 -->
                        <div class="glass-card p-4 rounded-2xl border border-slate-800/90 space-y-3 relative group hover:border-sky-500/50 transition-all">
                            <div class="flex items-center justify-between">
                                <span class="w-8 h-8 rounded-xl bg-sky-500/20 border border-sky-500/40 text-sky-400 font-extrabold text-xs flex items-center justify-center font-mono">03</span>
                                <i class="fa-solid fa-plus-circle text-slate-600 group-hover:text-sky-400 transition-colors text-sm"></i>
                            </div>
                            <h4 class="text-xs font-extrabold text-slate-100">3. Novo Webhook</h4>
                            <p class="text-[11px] text-slate-400 leading-relaxed">Clique no botão **+ Criar Webhook** (ou **Adicionar Webhook**) no topo da tela.</p>
                        </div>

                        <!-- Passo 4 -->
                        <div class="glass-card p-4 rounded-2xl border border-slate-800/90 space-y-3 relative group hover:border-sky-500/50 transition-all">
                            <div class="flex items-center justify-between">
                                <span class="w-8 h-8 rounded-xl bg-sky-500/20 border border-sky-500/40 text-sky-400 font-extrabold text-xs flex items-center justify-center font-mono">04</span>
                                <i class="fa-solid fa-link text-slate-600 group-hover:text-sky-400 transition-colors text-sm"></i>
                            </div>
                            <h4 class="text-xs font-extrabold text-slate-100">4. Cole a URL</h4>
                            <p class="text-[11px] text-slate-400 leading-relaxed">No campo **URL de Destino**, cole: `https://tubaraoia.lysia.tech/webhook`.</p>
                        </div>

                        <!-- Passo 5 -->
                        <div class="glass-card p-4 rounded-2xl border border-emerald-500/30 bg-emerald-950/10 space-y-3 relative group hover:border-emerald-500/60 transition-all">
                            <div class="flex items-center justify-between">
                                <span class="w-8 h-8 rounded-xl bg-emerald-500/20 border border-emerald-500/40 text-emerald-400 font-extrabold text-xs flex items-center justify-center font-mono">05</span>
                                <i class="fa-solid fa-circle-check text-emerald-400 text-sm"></i>
                            </div>
                            <h4 class="text-xs font-extrabold text-emerald-300">5. Ativar Eventos</h4>
                            <p class="text-[11px] text-slate-300 leading-relaxed">Marque os eventos **Mensagem (`Message`)** e **Chat (`Chat`)** e clique em **Salvar**!</p>
                        </div>
                    </div>
                </div>

                <!-- Testador Interativo ao Vivo do Webhook -->
                <div class="glass-panel p-6 rounded-3xl space-y-4 border border-slate-800">
                    <div class="flex flex-col sm:flex-row justify-between items-start sm:items-center gap-4 border-b border-slate-800 pb-4">
                        <div>
                            <h3 class="text-sm font-bold text-slate-100 flex items-center gap-2">
                                <i class="fa-solid fa-vial text-sky-400"></i>
                                <span>Testador de Conexão e Endpoint ao Vivo</span>
                            </h3>
                            <p class="text-xs text-slate-400">Verifique se o seu servidor Rust na VPS está online e pronto para receber notificações do uTalk</p>
                        </div>
                        <button @click="testWebhookEndpoint()" :disabled="testingWebhook"
                            class="px-4 py-2.5 bg-gradient-to-r from-emerald-600 to-teal-600 hover:from-emerald-500 hover:to-teal-500 text-white font-bold text-xs rounded-xl shadow-lg shadow-emerald-600/25 flex items-center gap-2 transition-all">
                            <i x-show="!testingWebhook" class="fa-solid fa-bolt"></i>
                            <i x-show="testingWebhook" class="fa-solid fa-circle-notch fa-spin"></i>
                            <span x-text="testingWebhook ? 'Testando Conexão...' : '⚡ Disparar Teste de Ping no Webhook'"></span>
                        </button>
                    </div>

                    <!-- Resultado do Teste ao Vivo -->
                    <div x-show="webhookTestResult" x-transition class="p-4 rounded-2xl text-xs font-mono border"
                        :class="webhookTestResultSuccess ? 'bg-emerald-950/60 border-emerald-800 text-emerald-300' : 'bg-rose-950/60 border-rose-800 text-rose-300'"
                        x-text="webhookTestResult"></div>
                </div>

                <!-- Painel com Exemplo de Payload JSON Formatado -->
                <div class="glass-panel p-6 rounded-3xl space-y-4 border border-slate-800">
                    <div class="flex items-center justify-between border-b border-slate-800 pb-3">
                        <h3 class="text-sm font-bold text-slate-100 flex items-center gap-2">
                            <i class="fa-solid fa-code text-indigo-400"></i>
                            <span>Formato dos Dados Enviados pelo uTalk (Payload JSON)</span>
                        </h3>
                        <span class="text-[10px] font-mono text-emerald-400 bg-emerald-950/60 border border-emerald-800/60 px-3 py-1 rounded-full font-bold">HTTPS POST /webhook</span>
                    </div>

                    <p class="text-xs text-slate-400">Toda vez que um cliente envia uma mensagem de texto ou áudio no WhatsApp, o uTalk envia uma requisição HTTP POST para a nossa URL com os dados abaixo:</p>

                    <pre class="bg-slate-950 p-5 rounded-2xl border border-slate-800 text-[11px] font-mono text-slate-300 overflow-x-auto selection:bg-indigo-500/30 leading-relaxed shadow-inner"><code>{
  "Event": "MessageCreated",
  "OrganizationId": "aORCMR51FFkJKvJe",
  "Payload": {
    "Type": "Message",
    "Content": {
      "Id": "aoEOqPVaoDSZJVGF",
      "MessageType": "Text", // "Text" para mensagens de texto ou "Audio" para mensagens de voz
      "Content": "Olá, preciso de um orçamento para bomba solar de 60 metros de profundidade",
      "Source": "Contact",
      "Chat": {
        "Id": "aoEOqPVaoDSZJVGF",
        "Contact": {
          "Name": "Cliente Exemplo",
          "PhoneNumber": "+5538999999999"
        }
      }
    }
  }
}</code></pre>
                </div>
            </div>
'''

    html = html[:tab_7_start] + new_tab_7_content + html[tab_7_end:]

# Add testing Webhook method in Alpine.js
old_js_methods = '                    copyWebhookUrl() {'
new_js_methods = '''                    testingWebhook: false,
                    webhookTestResult: '',
                    webhookTestResultSuccess: true,

                    async testWebhookEndpoint() {
                        this.testingWebhook = true;
                        this.webhookTestResult = '';
                        try {
                            const res = await fetch('/webhook', {
                                method: 'POST',
                                headers: { 'Content-Type': 'application/json' },
                                body: JSON.stringify({ Event: 'PingTest', Payload: { Type: 'Test' } })
                            });
                            if (res.ok) {
                                this.webhookTestResultSuccess = true;
                                this.webhookTestResult = '✅ SUCESSO! O servidor Webhook respondeu com HTTP 200 OK. A URL https://tubaraoia.lysia.tech/webhook está 100% ativa!';
                            } else {
                                this.webhookTestResultSuccess = false;
                                this.webhookTestResult = `⚠️ O servidor respondeu com o código HTTP ${res.status}.`;
                            }
                        } catch(err) {
                            this.webhookTestResultSuccess = false;
                            this.webhookTestResult = '❌ Falha ao se conectar com o endpoint /webhook.';
                        } finally {
                            this.testingWebhook = false;
                        }
                    },

                    copyWebhookUrl() {'''

if 'testWebhookEndpoint()' not in html:
    html = html.replace(old_js_methods, new_js_methods)

with open('templates/dashboard.html', 'w', encoding='utf-8') as f:
    f.write(html)

print('✅ Tela de Tutorial de Webhooks atualizada para um design PREMIUM e INTERATIVO!')

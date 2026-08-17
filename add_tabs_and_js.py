with open('templates/dashboard.html', 'r', encoding='utf-8') as f:
    html = f.read()

tabs_html = '''
            <!-- TAB 6: GESTÃO DE ACESSO & ADMINISTRADORES -->
            <div x-show="activeTab === 'access'" class="space-y-6" x-transition>
                <div class="grid grid-cols-1 lg:grid-cols-12 gap-6">
                    <!-- Card 1: Alterar Minha Senha -->
                    <div class="lg:col-span-5 glass-panel p-6 rounded-2xl space-y-4">
                        <div class="flex items-center gap-3 border-b border-slate-800 pb-4">
                            <div class="w-10 h-10 rounded-xl bg-cyan-500/15 border border-cyan-500/30 flex items-center justify-center">
                                <i class="fa-solid fa-key text-cyan-400 text-lg"></i>
                            </div>
                            <div>
                                <h3 class="text-base font-extrabold text-slate-100 glow-title">Alterar Minha Senha</h3>
                                <p class="text-xs text-slate-400">Atualize a senha de acesso da sua conta atual</p>
                            </div>
                        </div>

                        <form @submit.prevent="updatePassword()" class="space-y-4 pt-2">
                            <div>
                                <label class="block text-xs font-semibold text-slate-300 mb-1">Nova Senha</label>
                                <input type="password" x-model="newPassword" required placeholder="Digite a nova senha"
                                    class="w-full bg-slate-900/60 border border-slate-800 rounded-xl px-3.5 py-2.5 text-xs text-slate-200 focus:outline-none focus:border-cyan-500 transition-all">
                            </div>

                            <div>
                                <label class="block text-xs font-semibold text-slate-300 mb-1">Confirmar Nova Senha</label>
                                <input type="password" x-model="confirmPassword" required placeholder="Repita a nova senha"
                                    class="w-full bg-slate-900/60 border border-slate-800 rounded-xl px-3.5 py-2.5 text-xs text-slate-200 focus:outline-none focus:border-cyan-500 transition-all">
                            </div>

                            <button type="submit" :disabled="passwordLoading"
                                class="w-full py-3 bg-gradient-to-r from-cyan-600 to-indigo-600 hover:from-cyan-500 hover:to-indigo-500 text-white font-bold text-xs rounded-xl shadow-lg shadow-cyan-600/20 flex items-center justify-center gap-2 transition-all">
                                <span x-show="!passwordLoading"><i class="fa-solid fa-floppy-disk mr-1"></i> Salvar Nova Senha</span>
                                <i x-show="passwordLoading" class="fa-solid fa-circle-notch fa-spin"></i>
                            </button>
                        </form>
                    </div>

                    <!-- Card 2: Lista de Administradores Cadastrados -->
                    <div class="lg:col-span-7 glass-panel p-6 rounded-2xl space-y-4">
                        <div class="flex items-center justify-between border-b border-slate-800 pb-4">
                            <div class="flex items-center gap-3">
                                <div class="w-10 h-10 rounded-xl bg-indigo-500/15 border border-indigo-500/30 flex items-center justify-center">
                                    <i class="fa-solid fa-users-shield text-indigo-400 text-lg"></i>
                                </div>
                                <div>
                                    <h3 class="text-base font-extrabold text-slate-100 glow-title">Administradores do Sistema</h3>
                                    <p class="text-xs text-slate-400">Gerencie usuários com permissão de login no painel</p>
                                </div>
                            </div>
                            <button @click="showAddAdminForm = !showAddAdminForm"
                                class="px-3.5 py-2 bg-indigo-600/20 hover:bg-indigo-600/30 text-indigo-300 border border-indigo-500/30 rounded-xl text-xs font-bold transition-all flex items-center gap-1.5">
                                <i class="fa-solid" :class="showAddAdminForm ? 'fa-xmark' : 'fa-plus'"></i>
                                <span x-text="showAddAdminForm ? 'Cancelar' : '+ Novo Admin'"></span>
                            </button>
                        </div>

                        <!-- Form Inline: Adicionar Novo Administrador -->
                        <div x-show="showAddAdminForm" x-transition class="glass-card p-4 rounded-xl space-y-3 border border-indigo-500/30">
                            <h4 class="text-xs font-bold text-indigo-300 uppercase tracking-wider">Cadastrar Novo Administrador</h4>
                            <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
                                <div>
                                    <label class="block text-[11px] font-semibold text-slate-400 mb-1">Nome de Usuário</label>
                                    <input type="text" x-model="newAdminUsername" placeholder="ex: leandro"
                                        class="w-full bg-slate-950 border border-slate-800 rounded-xl px-3 py-2 text-xs text-slate-200 focus:outline-none focus:border-indigo-500">
                                </div>
                                <div>
                                    <label class="block text-[11px] font-semibold text-slate-400 mb-1">Senha de Acesso</label>
                                    <input type="password" x-model="newAdminPassword" placeholder="Senha segura"
                                        class="w-full bg-slate-950 border border-slate-800 rounded-xl px-3 py-2 text-xs text-slate-200 focus:outline-none focus:border-indigo-500">
                                </div>
                            </div>
                            <button @click="addAdminUser()" :disabled="addUserLoading"
                                class="w-full py-2.5 bg-indigo-600 hover:bg-indigo-500 text-white font-bold text-xs rounded-xl transition-all shadow-md flex items-center justify-center gap-2">
                                <span x-show="!addUserLoading">Confirmar Cadastro</span>
                                <i x-show="addUserLoading" class="fa-solid fa-circle-notch fa-spin"></i>
                            </button>
                        </div>

                        <!-- Tabela de Admins -->
                        <div class="overflow-x-auto">
                            <table class="w-full text-left text-xs text-slate-300">
                                <thead class="bg-slate-900/80 text-slate-400 font-mono uppercase text-[10px]">
                                    <tr>
                                        <th class="p-3">ID</th>
                                        <th class="p-3">Usuário</th>
                                        <th class="p-3">Data de Criação</th>
                                        <th class="p-3 text-right">Ação</th>
                                    </tr>
                                </thead>
                                <tbody class="divide-y divide-slate-800/60">
                                    <template x-for="user in adminUsers" :key="user.id">
                                        <tr class="hover:bg-slate-900/50">
                                            <td class="p-3 font-mono text-slate-500" x-text="'#' + user.id"></td>
                                            <td class="p-3 font-bold text-slate-200" x-text="user.username"></td>
                                            <td class="p-3 font-mono text-slate-400 text-[11px]" x-text="user.created_at || 'Sistema'"></td>
                                            <td class="p-3 text-right">
                                                <button @click="deleteAdminUser(user.id, user.username)"
                                                    class="px-2.5 py-1 bg-rose-950/40 hover:bg-rose-900/60 text-rose-400 border border-rose-900/40 rounded-lg text-[11px] font-bold transition-all">
                                                    Excluir
                                                </button>
                                            </td>
                                        </tr>
                                    </template>
                                </tbody>
                            </table>
                        </div>
                    </div>
                </div>
            </div>

            <!-- TAB 7: TUTORIAL & CONFIGURAÇÃO DE WEBHOOKS -->
            <div x-show="activeTab === 'webhooks'" class="space-y-6" x-transition>
                <!-- Hero Webhook Container -->
                <div class="glass-panel p-6 md:p-8 rounded-2xl space-y-6 border border-sky-500/30 relative overflow-hidden">
                    <div class="flex flex-col md:flex-row justify-between items-start md:items-center gap-4">
                        <div class="space-y-2">
                            <div class="flex items-center gap-3">
                                <div class="w-10 h-10 rounded-xl bg-sky-500/20 border border-sky-400/40 flex items-center justify-center shrink-0">
                                    <i class="fa-solid fa-satellite-dish text-sky-400 text-lg"></i>
                                </div>
                                <div>
                                    <h2 class="text-xl font-extrabold text-slate-100 glow-title">Tutorial de Integração de Webhook uTalk</h2>
                                    <p class="text-xs text-slate-400">Como conectar o robô de IA no seu painel oficial da Umbler</p>
                                </div>
                            </div>
                        </div>

                        <!-- URL do Webhook em Destaque -->
                        <div class="glass-card p-3 rounded-xl border border-sky-500/40 bg-slate-950/80 w-full md:w-auto">
                            <span class="block text-[10px] font-mono uppercase text-sky-400 font-bold mb-1">URL Oficial do Webhook:</span>
                            <div class="flex items-center gap-2">
                                <code class="text-xs font-mono text-emerald-400 bg-slate-900 px-3 py-1.5 rounded-lg border border-slate-800 selection:bg-indigo-500/40" id="webhookUrlText">https://tubaraoia.lysia.tech/webhook</code>
                                <button @click="copyWebhookUrl()" class="px-3 py-1.5 bg-sky-600 hover:bg-sky-500 text-white rounded-lg text-xs font-bold transition-all flex items-center gap-1 shadow-md">
                                    <i class="fa-solid fa-copy"></i>
                                    <span>Copiar</span>
                                </button>
                            </div>
                        </div>
                    </div>

                    <!-- Guias / Passos Numerados -->
                    <div class="grid grid-cols-1 md:grid-cols-4 gap-4 pt-2">
                        <div class="glass-card p-4 rounded-xl border border-slate-800 space-y-2">
                            <span class="w-7 h-7 rounded-full bg-sky-500/20 text-sky-400 font-bold text-xs flex items-center justify-center font-mono">1</span>
                            <h4 class="text-xs font-bold text-slate-200">Acesse o uTalk</h4>
                            <p class="text-[11px] text-slate-400 leading-relaxed">Entre na sua conta uTalk (`app-utalk.umbler.com`) e navegue até **Configurações -> Webhooks**.</p>
                        </div>
                        <div class="glass-card p-4 rounded-xl border border-slate-800 space-y-2">
                            <span class="w-7 h-7 rounded-full bg-sky-500/20 text-sky-400 font-bold text-xs flex items-center justify-center font-mono">2</span>
                            <h4 class="text-xs font-bold text-slate-200">Criar Novo Webhook</h4>
                            <p class="text-[11px] text-slate-400 leading-relaxed">Clique no botão **+ Criar Webhook** no canto superior direito.</p>
                        </div>
                        <div class="glass-card p-4 rounded-xl border border-slate-800 space-y-2">
                            <span class="w-7 h-7 rounded-full bg-sky-500/20 text-sky-400 font-bold text-xs flex items-center justify-center font-mono">3</span>
                            <h4 class="text-xs font-bold text-slate-200">Cole a URL de Destino</h4>
                            <p class="text-[11px] text-slate-400 leading-relaxed">Cole a URL `https://tubaraoia.lysia.tech/webhook` no campo **URL do Webhook**.</p>
                        </div>
                        <div class="glass-card p-4 rounded-xl border border-slate-800 space-y-2">
                            <span class="w-7 h-7 rounded-full bg-emerald-500/20 text-emerald-400 font-bold text-xs flex items-center justify-center font-mono">4</span>
                            <h4 class="text-xs font-bold text-slate-200">Selecione os Eventos</h4>
                            <p class="text-[11px] text-slate-400 leading-relaxed">Marque os eventos **Mensagem (`Message`)** e **Chat (`Chat`)** e clique em **Salvar**.</p>
                        </div>
                    </div>
                </div>

                <!-- Exemplo de JSON Webhook Payload -->
                <div class="glass-panel p-6 rounded-2xl space-y-4">
                    <div class="flex items-center justify-between border-b border-slate-800 pb-3">
                        <h3 class="text-sm font-bold text-slate-100 flex items-center gap-2">
                            <i class="fa-solid fa-code text-indigo-400"></i>
                            <span>Estrutura do Payload JSON Recebido pelo uTalk</span>
                        </h3>
                        <span class="text-[10px] font-mono text-emerald-400 bg-emerald-950/60 border border-emerald-800/60 px-2.5 py-1 rounded-full">HTTPS POST /webhook</span>
                    </div>
                    <pre class="bg-slate-950 p-4 rounded-xl border border-slate-800 text-[11px] font-mono text-slate-300 overflow-x-auto selection:bg-indigo-500/30"><code>{
  "Event": "MessageCreated",
  "Payload": {
    "Type": "Message",
    "Content": {
      "Id": "aoEOqPVaoDSZJVGF",
      "MessageType": "Text", // ou "Audio"
      "Content": "Olá, gostaria de saber o valor para bomba solar de 60 metros",
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

# Insert tabs_html before script
script_marker = '<!-- Script Alpine.js Reativo com Estado da Sidebar Retrátil -->'
if script_marker in html:
    html = html.replace(script_marker, tabs_html + '\n\n        ' + script_marker)

# Update Alpine.js data & methods
old_app_data = '''                    alert: { show: false, msg: '', type: 'success' },
                    apiModalOpen: false,'''

new_app_data = '''                    alert: { show: false, msg: '', type: 'success' },
                    apiModalOpen: false,
                    showAddAdminForm: false,
                    adminUsers: [],
                    newAdminUsername: '',
                    newAdminPassword: '',
                    newPassword: '',
                    confirmPassword: '',
                    passwordLoading: false,
                    addUserLoading: false,'''

if 'showAddAdminForm' not in html:
    html = html.replace(old_app_data, new_app_data)

# Update switchTab logic in JS
old_switch_tab = '''                    switchTab(tabId) {
                        this.activeTab = tabId;
                        if (tabId === 'metrics') this.loadStats();
                        if (tabId === 'rotation' && this.operators.length === 0) this.fetchOperators();
                    },'''

new_switch_tab = '''                    switchTab(tabId) {
                        this.activeTab = tabId;
                        if (tabId === 'metrics') this.loadStats();
                        if (tabId === 'rotation' && this.operators.length === 0) this.fetchOperators();
                        if (tabId === 'access') this.loadAdminUsers();
                    },'''

if "tabId === 'access'" not in html:
    html = html.replace(old_switch_tab, new_switch_tab)

# Add admin & password methods to JS object before doLogout
old_logout = '                    async doLogout() {'

new_methods = '''                    async loadAdminUsers() {
                        try {
                            const res = await fetch('/api/users');
                            if (res.ok) {
                                this.adminUsers = await res.json();
                            }
                        } catch (err) { console.error('Erro ao carregar admins:', err); }
                    },

                    async addAdminUser() {
                        if (!this.newAdminUsername.trim() || !this.newAdminPassword.trim()) {
                            this.showAlert('Preencha o nome do usuário e a senha.', 'error');
                            return;
                        }
                        this.addUserLoading = true;
                        try {
                            const res = await fetch('/api/users', {
                                method: 'POST',
                                headers: { 'Content-Type': 'application/json' },
                                body: JSON.stringify({ username: this.newAdminUsername, password: this.newAdminPassword })
                            });
                            const data = await res.json();
                            if (res.ok && data.success) {
                                this.showAlert('Administrador cadastrado com sucesso!');
                                this.newAdminUsername = '';
                                this.newAdminPassword = '';
                                this.showAddAdminForm = false;
                                this.loadAdminUsers();
                            } else {
                                this.showAlert(data.error || 'Erro ao cadastrar administrador.', 'error');
                            }
                        } catch (err) { this.showAlert('Falha na comunicação com o servidor.', 'error'); }
                        finally { this.addUserLoading = false; }
                    },

                    async deleteAdminUser(id, username) {
                        if (!confirm(`Deseja realmente excluir o administrador "${username}"?`)) return;
                        try {
                            const res = await fetch(`/api/users/${id}`, { method: 'DELETE' });
                            const data = await res.json();
                            if (res.ok && data.success) {
                                this.showAlert('Administrador excluído com sucesso!');
                                this.loadAdminUsers();
                            } else {
                                this.showAlert(data.error || 'Erro ao excluir administrador.', 'error');
                            }
                        } catch (err) { this.showAlert('Falha na comunicação com o servidor.', 'error'); }
                    },

                    async updatePassword() {
                        if (!this.newPassword.trim()) {
                            this.showAlert('Digite a nova senha.', 'error');
                            return;
                        }
                        if (this.newPassword !== this.confirmPassword) {
                            this.showAlert('A confirmação de senha não confere.', 'error');
                            return;
                        }
                        this.passwordLoading = true;
                        try {
                            const res = await fetch('/api/change-password', {
                                method: 'POST',
                                headers: { 'Content-Type': 'application/json' },
                                body: JSON.stringify({ new_password: this.newPassword })
                            });
                            const data = await res.json();
                            if (res.ok && data.success) {
                                this.showAlert('Senha alterada com sucesso!');
                                this.newPassword = '';
                                this.confirmPassword = '';
                            } else {
                                this.showAlert(data.error || 'Erro ao alterar senha.', 'error');
                            }
                        } catch (err) { this.showAlert('Falha na comunicação com o servidor.', 'error'); }
                        finally { this.passwordLoading = false; }
                    },

                    copyWebhookUrl() {
                        const url = 'https://tubaraoia.lysia.tech/webhook';
                        navigator.clipboard.writeText(url);
                        this.showAlert('URL do Webhook copiada para a área de transferência!');
                    },

''' + old_logout

if 'loadAdminUsers()' not in html:
    html = html.replace(old_logout, new_methods)

with open('templates/dashboard.html', 'w', encoding='utf-8') as f:
    f.write(html)

print('✅ Script de inclusão das abas Gestão de Acesso e Webhooks finalizado com sucesso!')

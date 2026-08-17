import re

with open('templates/dashboard.html', 'r', encoding='utf-8') as f:
    html = f.read()

# 1. Update the "Cadastrar Novo Administrador" Card HTML in TAB 6 with Confirm Password & Legend
old_admin_card = '''                        <!-- Card 1: Criar Novo Administrador (Sempre Visível) -->
                        <div class="glass-panel p-6 rounded-3xl space-y-4 border border-indigo-500/30 bg-slate-900/90 shadow-xl">
                            <div class="flex items-center gap-3 border-b border-slate-800 pb-4">
                                <div class="w-10 h-10 rounded-xl bg-indigo-500/20 border border-indigo-500/40 flex items-center justify-center shrink-0">
                                    <i class="fa-solid fa-user-plus text-indigo-400 text-lg"></i>
                                </div>
                                <div>
                                    <h3 class="text-base font-extrabold text-slate-100 glow-title">Cadastrar Novo Administrador</h3>
                                    <p class="text-xs text-slate-400">Adicione outro usuário com permissão de login no painel</p>
                                </div>
                            </div>

                            <div class="space-y-4 pt-1">
                                <div>
                                    <label class="block text-xs font-semibold text-slate-300 mb-1">Nome de Usuário (Login)</label>
                                    <input type="text" x-model="newAdminUsername" placeholder="ex: leandro"
                                        class="w-full bg-slate-950 border border-slate-800 rounded-xl px-3.5 py-2.5 text-xs text-slate-200 focus:outline-none focus:border-indigo-500 font-medium">
                                </div>

                                <div>
                                    <label class="block text-xs font-semibold text-slate-300 mb-1">Senha de Acesso</label>
                                    <input type="password" x-model="newAdminPassword" placeholder="Digite uma senha segura"
                                        class="w-full bg-slate-950 border border-slate-800 rounded-xl px-3.5 py-2.5 text-xs text-slate-200 focus:outline-none focus:border-indigo-500">
                                </div>

                                <button @click="addAdminUser()" :disabled="addUserLoading"
                                    class="w-full py-3 bg-gradient-to-r from-indigo-600 to-purple-600 hover:from-indigo-500 hover:to-purple-500 text-white font-bold text-xs rounded-xl shadow-lg shadow-indigo-600/25 flex items-center justify-center gap-2 transition-all">
                                    <span x-show="!addUserLoading"><i class="fa-solid fa-user-check mr-1"></i> Cadastrar Administrador</span>
                                    <i x-show="addUserLoading" class="fa-solid fa-circle-notch fa-spin"></i>
                                </button>
                            </div>
                        </div>'''

new_admin_card = '''                        <!-- Card 1: Criar Novo Administrador (Sempre Visível) -->
                        <div class="glass-panel p-6 rounded-3xl space-y-4 border border-indigo-500/30 bg-slate-900/90 shadow-xl">
                            <div class="flex items-center gap-3 border-b border-slate-800 pb-4">
                                <div class="w-10 h-10 rounded-xl bg-indigo-500/20 border border-indigo-500/40 flex items-center justify-center shrink-0">
                                    <i class="fa-solid fa-user-plus text-indigo-400 text-lg"></i>
                                </div>
                                <div>
                                    <h3 class="text-base font-extrabold text-slate-100 glow-title">Cadastrar Novo Administrador</h3>
                                    <p class="text-xs text-slate-400">Crie outro login de acesso ao painel para a sua equipe</p>
                                </div>
                            </div>

                            <form @submit.prevent="addAdminUser()" class="space-y-4 pt-1">
                                <div>
                                    <label class="block text-xs font-semibold text-slate-300 mb-1">Nome de Usuário (Login)</label>
                                    <input type="text" x-model="newAdminUsername" required placeholder="ex: leandro" autocomplete="username"
                                        class="w-full bg-slate-950 border border-slate-800 rounded-xl px-3.5 py-2.5 text-xs text-slate-200 focus:outline-none focus:border-indigo-500 font-medium">
                                </div>

                                <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
                                    <div>
                                        <label class="block text-xs font-semibold text-slate-300 mb-1">Senha de Acesso</label>
                                        <input type="password" x-model="newAdminPassword" required placeholder="Digite a senha" autocomplete="new-password"
                                            class="w-full bg-slate-950 border border-slate-800 rounded-xl px-3.5 py-2.5 text-xs text-slate-200 focus:outline-none focus:border-indigo-500">
                                    </div>
                                    <div>
                                        <label class="block text-xs font-semibold text-slate-300 mb-1">Confirmar Senha</label>
                                        <input type="password" x-model="confirmAdminPassword" required placeholder="Repita a senha" autocomplete="new-password"
                                            class="w-full bg-slate-950 border border-slate-800 rounded-xl px-3.5 py-2.5 text-xs text-slate-200 focus:outline-none focus:border-indigo-500">
                                    </div>
                                </div>

                                <p class="text-[11px] text-slate-400 bg-indigo-950/40 p-2.5 rounded-xl border border-indigo-900/40 flex items-center gap-2">
                                    <i class="fa-solid fa-shield-halved text-indigo-400"></i>
                                    <span>Este novo administrador terá acesso completo para gerenciar robôs, prompt e atendentes no painel.</span>
                                </p>

                                <button type="submit" :disabled="addUserLoading"
                                    class="w-full py-3 bg-gradient-to-r from-indigo-600 to-purple-600 hover:from-indigo-500 hover:to-purple-500 text-white font-bold text-xs rounded-xl shadow-lg shadow-indigo-600/25 flex items-center justify-center gap-2 transition-all">
                                    <span x-show="!addUserLoading"><i class="fa-solid fa-user-plus mr-1"></i> Confirmar e Cadastrar Novo Administrador</span>
                                    <i x-show="addUserLoading" class="fa-solid fa-circle-notch fa-spin"></i>
                                </button>
                            </form>
                        </div>'''

if old_admin_card in html:
    html = html.replace(old_admin_card, new_admin_card)

# 2. Fix autocomplete on other password inputs to eliminate DOM warnings
html = html.replace('x-model="newPassword" required placeholder="Digite a nova senha"', 'x-model="newPassword" required placeholder="Digite a nova senha" autocomplete="new-password"')
html = html.replace('x-model="confirmPassword" required placeholder="Repita a nova senha"', 'x-model="confirmPassword" required placeholder="Repita a nova senha" autocomplete="new-password"')

# 3. Update dashboardApp JS object state and methods
old_app_def = '''            function dashboardApp() {
                return {
                    loggedIn: false,
                    loginUsername: '',
                    loginPassword: '',
                    loginLoading: false,
                    loginError: '',
                    activeTab: 'metrics','''

new_app_def = '''            function dashboardApp() {
                return {
                    loggedIn: false,
                    loginUsername: '',
                    loginPassword: '',
                    loggedUser: '',
                    loginLoading: false,
                    loginError: '',
                    activeTab: 'metrics',
                    adminUsers: [],
                    newAdminUsername: '',
                    newAdminPassword: '',
                    confirmAdminPassword: '',
                    addUserLoading: false,
                    newPassword: '',
                    confirmPassword: '',
                    passwordLoading: false,
                    testingWebhook: false,
                    webhookTestResult: '',
                    webhookTestResultSuccess: true,'''

if old_app_def in html:
    html = html.replace(old_app_def, new_app_def)

# 4. Insert all JS methods into dashboardApp object before doLogout
old_do_logout = '                    async doLogout() {'

full_methods = '''                    async loadAdminUsers() {
                        try {
                            const res = await fetch('/api/users');
                            if (res.ok) {
                                this.adminUsers = await res.json();
                            }
                        } catch (err) {
                            console.error('Erro ao carregar lista de administradores:', err);
                        }
                    },

                    async addAdminUser() {
                        if (!this.newAdminUsername.trim() || !this.newAdminPassword.trim()) {
                            this.showAlert('Preencha o nome de usuário e a senha.', 'error');
                            return;
                        }
                        if (this.newAdminPassword !== this.confirmAdminPassword) {
                            this.showAlert('A confirmação de senha do novo administrador não confere.', 'error');
                            return;
                        }
                        this.addUserLoading = true;
                        try {
                            const res = await fetch('/api/users', {
                                method: 'POST',
                                headers: { 'Content-Type': 'application/json' },
                                body: JSON.stringify({ username: this.newAdminUsername.trim(), password: this.newAdminPassword.trim() })
                            });
                            const data = await res.json();
                            if (res.ok && data.success) {
                                this.showAlert('Novo administrador cadastrado com sucesso!');
                                this.newAdminUsername = '';
                                this.newAdminPassword = '';
                                this.confirmAdminPassword = '';
                                this.loadAdminUsers();
                            } else {
                                this.showAlert(data.error || 'Erro ao cadastrar administrador.', 'error');
                            }
                        } catch (err) {
                            this.showAlert('Falha na comunicação com o servidor.', 'error');
                        } finally {
                            this.addUserLoading = false;
                        }
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
                        } catch (err) {
                            this.showAlert('Falha na comunicação com o servidor.', 'error');
                        }
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
                                body: JSON.stringify({ new_password: this.newPassword.trim() })
                            });
                            const data = await res.json();
                            if (res.ok && data.success) {
                                this.showAlert('Senha alterada com sucesso!');
                                this.newPassword = '';
                                this.confirmPassword = '';
                            } else {
                                this.showAlert(data.error || 'Erro ao alterar senha.', 'error');
                            }
                        } catch (err) {
                            this.showAlert('Falha na comunicação com o servidor.', 'error');
                        } finally {
                            this.passwordLoading = false;
                        }
                    },

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
                                this.webhookTestResult = '✅ SUCESSO! O servidor Webhook respondeu com HTTP 200 OK. O seu endpoint https://tubaraoia.lysia.tech/webhook está 100% ativo!';
                            } else {
                                this.webhookTestResultSuccess = false;
                                this.webhookTestResult = `⚠️ O servidor respondeu com código HTTP ${res.status}.`;
                            }
                        } catch(err) {
                            this.webhookTestResultSuccess = false;
                            this.webhookTestResult = '❌ Falha ao se conectar com o endpoint /webhook.';
                        } finally {
                            this.testingWebhook = false;
                        }
                    },

                    copyWebhookUrl() {
                        const url = 'https://tubaraoia.lysia.tech/webhook';
                        navigator.clipboard.writeText(url);
                        this.showAlert('URL do Webhook copiada para a área de transferência!');
                    },

''' + old_do_logout

if 'loadAdminUsers()' not in html:
    html = html.replace(old_do_logout, full_methods)

with open('templates/dashboard.html', 'w', encoding='utf-8') as f:
    f.write(html)

print('✅ Script de correção do Alpine JS + Confirmação de Senha + Legendas finalizado!')

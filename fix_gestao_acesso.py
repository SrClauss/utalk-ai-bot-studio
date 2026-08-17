import re

with open('templates/dashboard.html', 'r', encoding='utf-8') as f:
    html = f.read()

# 1. Make Banner Hero display ONLY on metrics tab or be compact
old_hero = '<!-- Banner Hero Integrado com Imagem Neuronal (/assets/banner.jpg) -->'
new_hero = '<!-- Banner Hero Integrado (Visível na aba Métricas) -->\n            <div x-show="activeTab === \'metrics\'" class="relative overflow-hidden rounded-2xl border border-indigo-500/30 bg-slate-900/90 shadow-2xl">'

if old_hero in html and 'x-show="activeTab === \'metrics\'"' not in html:
    html = html.replace(old_hero, new_hero)
    # Match the end of that div
    html = html.replace(
        '<!-- Banner de Alertas (Alpine.js Reativo) -->',
        '</div>\n\n            <!-- Banner de Alertas (Alpine.js Reativo) -->'
    )

# 2. Complete Redesign of TAB 6: GESTÃO DE ACESSO
old_tab_6 = '''            <!-- TAB 6: GESTÃO DE ACESSO & ADMINISTRADORES -->'''
tab_6_start = html.find(old_tab_6)
tab_6_end = html.find('<!-- TAB 7: TUTORIAL & CONFIGURAÇÃO DE WEBHOOKS -->')

if tab_6_start != -1 and tab_6_end != -1:
    new_tab_6 = '''            <!-- TAB 6: GESTÃO DE ACESSO & ADMINISTRADORES -->
            <div x-show="activeTab === 'access'" class="space-y-6" x-transition>
                
                <!-- Perfil Atual / Quem Eu Sou -->
                <div class="glass-panel p-6 rounded-3xl border border-cyan-500/30 flex flex-col md:flex-row justify-between items-start md:items-center gap-4 bg-slate-900/80 shadow-xl">
                    <div class="flex items-center gap-4">
                        <div class="w-14 h-14 rounded-2xl bg-gradient-to-tr from-cyan-500 to-indigo-600 flex items-center justify-center text-white text-2xl shadow-lg shadow-cyan-500/25 shrink-0">
                            <i class="fa-solid fa-user-shield"></i>
                        </div>
                        <div>
                            <div class="flex items-center gap-2">
                                <span class="text-xs font-mono uppercase tracking-wider text-cyan-400 font-bold">Você está logado como:</span>
                                <span class="text-xs font-mono font-bold bg-cyan-500/20 text-cyan-300 border border-cyan-500/30 px-2 py-0.5 rounded-full">Sessão Ativa</span>
                            </div>
                            <h2 class="text-2xl font-black text-slate-100 glow-title tracking-tight" x-text="loggedUser || loginUsername || 'admin'">admin</h2>
                            <p class="text-xs text-slate-400">Permissão de Administrador Master do Sistema uTalk AI Bot</p>
                        </div>
                    </div>

                    <button @click="doLogout()" class="px-4 py-2.5 bg-rose-950/40 hover:bg-rose-900/60 text-rose-300 border border-rose-900/40 rounded-xl text-xs font-bold transition-all flex items-center gap-2">
                        <i class="fa-solid fa-right-from-bracket text-rose-400"></i>
                        <span>Encerrar Sessão</span>
                    </button>
                </div>

                <div class="grid grid-cols-1 lg:grid-cols-12 gap-6">
                    <!-- Coluna Esquerda: Form de Criar Admin + Alterar Senha -->
                    <div class="lg:col-span-6 space-y-6">
                        
                        <!-- Card 1: Criar Novo Administrador (Sempre Visível) -->
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
                        </div>

                        <!-- Card 2: Alterar Minha Senha -->
                        <div class="glass-panel p-6 rounded-3xl space-y-4 border border-slate-800 bg-slate-900/80">
                            <div class="flex items-center gap-3 border-b border-slate-800 pb-4">
                                <div class="w-10 h-10 rounded-xl bg-cyan-500/15 border border-cyan-500/30 flex items-center justify-center shrink-0">
                                    <i class="fa-solid fa-key text-cyan-400 text-lg"></i>
                                </div>
                                <div>
                                    <h3 class="text-base font-extrabold text-slate-100">Alterar Minha Senha</h3>
                                    <p class="text-xs text-slate-400">Troque a senha da sua conta atual</p>
                                </div>
                            </div>

                            <form @submit.prevent="updatePassword()" class="space-y-4 pt-1">
                                <div>
                                    <label class="block text-xs font-semibold text-slate-300 mb-1">Nova Senha</label>
                                    <input type="password" x-model="newPassword" required placeholder="Digite a nova senha"
                                        class="w-full bg-slate-950 border border-slate-800 rounded-xl px-3.5 py-2.5 text-xs text-slate-200 focus:outline-none focus:border-cyan-500 transition-all">
                                </div>

                                <div>
                                    <label class="block text-xs font-semibold text-slate-300 mb-1">Confirmar Nova Senha</label>
                                    <input type="password" x-model="confirmPassword" required placeholder="Repita a nova senha"
                                        class="w-full bg-slate-950 border border-slate-800 rounded-xl px-3.5 py-2.5 text-xs text-slate-200 focus:outline-none focus:border-cyan-500 transition-all">
                                </div>

                                <button type="submit" :disabled="passwordLoading"
                                    class="w-full py-3 bg-gradient-to-r from-cyan-600 to-indigo-600 hover:from-cyan-500 hover:to-indigo-500 text-white font-bold text-xs rounded-xl shadow-md flex items-center justify-center gap-2 transition-all">
                                    <span x-show="!passwordLoading"><i class="fa-solid fa-floppy-disk mr-1"></i> Salvar Nova Senha</span>
                                    <i x-show="passwordLoading" class="fa-solid fa-circle-notch fa-spin"></i>
                                </button>
                            </form>
                        </div>
                    </div>

                    <!-- Coluna Direita: Tabela de Administradores Cadastrados -->
                    <div class="lg:col-span-6 glass-panel p-6 rounded-3xl space-y-4 border border-slate-800 flex flex-col">
                        <div class="flex items-center justify-between border-b border-slate-800 pb-4">
                            <div class="flex items-center gap-3">
                                <div class="w-10 h-10 rounded-xl bg-purple-500/15 border border-purple-500/30 flex items-center justify-center shrink-0">
                                    <i class="fa-solid fa-users-shield text-purple-400 text-lg"></i>
                                </div>
                                <div>
                                    <h3 class="text-base font-extrabold text-slate-100 glow-title">Administradores Cadastrados</h3>
                                    <p class="text-xs text-slate-400">Lista completa de usuários com acesso ao painel</p>
                                </div>
                            </div>
                            <button @click="loadAdminUsers()" class="text-xs text-indigo-400 hover:underline flex items-center gap-1">
                                <i class="fa-solid fa-rotate-right"></i>
                                <span>Atualizar</span>
                            </button>
                        </div>

                        <!-- Tabela de Admins -->
                        <div class="overflow-x-auto flex-1">
                            <table class="w-full text-left text-xs text-slate-300">
                                <thead class="bg-slate-900/90 text-slate-400 font-mono uppercase text-[10px]">
                                    <tr>
                                        <th class="p-3">Usuário</th>
                                        <th class="p-3">Criado em</th>
                                        <th class="p-3 text-right">Ação</th>
                                    </tr>
                                </thead>
                                <tbody class="divide-y divide-slate-800/60">
                                    <template x-for="user in adminUsers" :key="user.id">
                                        <tr class="hover:bg-slate-900/50 transition-colors">
                                            <td class="p-3 font-bold text-slate-100 flex items-center gap-2">
                                                <i class="fa-solid fa-user-circle text-indigo-400 text-sm"></i>
                                                <span x-text="user.username"></span>
                                                <span x-show="user.username.toLowerCase() === (loggedUser || loginUsername || '').toLowerCase()"
                                                    class="text-[9px] bg-cyan-500/20 text-cyan-300 border border-cyan-500/40 px-1.5 py-0.2 rounded font-bold">VOCÊ</span>
                                            </td>
                                            <td class="p-3 font-mono text-slate-400 text-[11px]" x-text="user.created_at || 'Sistema'"></td>
                                            <td class="p-3 text-right">
                                                <button x-show="user.username.toLowerCase() !== (loggedUser || loginUsername || '').toLowerCase()"
                                                    @click="deleteAdminUser(user.id, user.username)"
                                                    class="px-3 py-1.5 bg-rose-950/40 hover:bg-rose-900/60 text-rose-300 border border-rose-900/40 rounded-lg text-xs font-bold transition-all shadow-sm">
                                                    <i class="fa-solid fa-trash-can mr-1"></i> Excluir
                                                </button>
                                                <span x-show="user.username.toLowerCase() === (loggedUser || loginUsername || '').toLowerCase()"
                                                    class="text-[10px] text-slate-500 font-mono italic">Conta Atual</span>
                                            </td>
                                        </tr>
                                    </template>
                                </tbody>
                            </table>
                        </div>
                    </div>
                </div>
            </div>

            '''
    html = html[:tab_6_start] + new_tab_6 + html[tab_6_end:]

# Store logged user in state
if 'loggedUser:' not in html:
    html = html.replace('loginUsername: \'\',', 'loginUsername: \'\',\n                    loggedUser: \'\',')
    html = html.replace('this.loggedIn = true;\n                                this.loginPassword = \'\';', 'this.loggedIn = true;\n                                this.loggedUser = this.loginUsername;\n                                this.loginPassword = \'\';')

with open('templates/dashboard.html', 'w', encoding='utf-8') as f:
    f.write(html)

print('✅ Redesenho da aba Gestão de Acesso concluído com sucesso!')

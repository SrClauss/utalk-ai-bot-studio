import re

with open('templates/dashboard.html', 'r', encoding='utf-8') as f:
    html = f.read()

# 1. Add overflow-x fix to head style
if 'overflow-x: hidden !important;' not in html:
    html = html.replace(
        "font-family: 'Inter', sans-serif;",
        "overflow-x: hidden !important;\n            max-width: 100vw;\n            font-family: 'Inter', sans-serif;"
    )

# 2. Fix main element margin to padding
html = html.replace(
    ":class=\"sidebarCollapsed ? 'md:ml-20' : 'md:ml-64'\"",
    ":class=\"sidebarCollapsed ? 'md:pl-20' : 'md:pl-64'\""
)
html = html.replace(
    "class=\"transition-all duration-300 flex-1 w-full px-4 sm:px-6",
    "class=\"transition-all duration-300 flex-1 w-full max-w-full overflow-x-hidden px-4 sm:px-6"
)

# 3. Add sidebar nav buttons
target_btn = '<span x-show="!sidebarCollapsed" x-transition class="truncate">APIs Externas & Tools</span>\n                </button>'
new_btns = target_btn + '''

                <button @click="switchTab('access')"
                    :class="{ 'active': activeTab === 'access', 'justify-center px-0': sidebarCollapsed, 'justify-start px-2': !sidebarCollapsed }"
                    class="sidebar-item w-full flex items-center gap-3 py-2 rounded-xl text-xs font-semibold text-slate-400 hover:text-white hover:bg-slate-900/60 group"
                    :title="sidebarCollapsed ? 'Gestão de Acesso' : ''">
                    <div
                        class="w-9 h-9 rounded-xl bg-cyan-500/15 border border-cyan-500/30 flex items-center justify-center shrink-0 group-hover:scale-105 transition-transform shadow-sm">
                        <i class="fa-solid fa-user-shield text-cyan-400 text-sm"></i>
                    </div>
                    <span x-show="!sidebarCollapsed" x-transition class="truncate">Gestão de Acesso</span>
                </button>

                <button @click="switchTab('webhooks')"
                    :class="{ 'active': activeTab === 'webhooks', 'justify-center px-0': sidebarCollapsed, 'justify-start px-2': !sidebarCollapsed }"
                    class="sidebar-item w-full flex items-center gap-3 py-2 rounded-xl text-xs font-semibold text-slate-400 hover:text-white hover:bg-slate-900/60 group"
                    :title="sidebarCollapsed ? 'Tutorial Webhooks' : ''">
                    <div
                        class="w-9 h-9 rounded-xl bg-sky-500/15 border border-sky-500/30 flex items-center justify-center shrink-0 group-hover:scale-105 transition-transform shadow-sm">
                        <i class="fa-solid fa-satellite-dish text-sky-400 text-sm"></i>
                    </div>
                    <span x-show="!sidebarCollapsed" x-transition class="truncate">Tutorial Webhooks</span>
                </button>'''

if 'switchTab(\'access\')' not in html:
    html = html.replace(target_btn, new_btns)

# 4. Add Mobile Bottom Nav items
mobile_nav_target = '''            <button @click="switchTab('apis')"
                :class="{ 'text-indigo-400 font-bold': activeTab === 'apis', 'text-slate-400': activeTab !== 'apis' }"
                class="flex flex-col items-center gap-1 px-3 py-1 rounded-xl transition-all">
                <i class="fa-solid fa-plug text-base"></i>
                <span>APIs</span>
            </button>'''

mobile_nav_new = mobile_nav_target + '''
            <button @click="switchTab('access')"
                :class="{ 'text-indigo-400 font-bold': activeTab === 'access', 'text-slate-400': activeTab !== 'access' }"
                class="flex flex-col items-center gap-1 px-3 py-1 rounded-xl transition-all">
                <i class="fa-solid fa-user-shield text-base"></i>
                <span>Acesso</span>
            </button>
            <button @click="switchTab('webhooks')"
                :class="{ 'text-indigo-400 font-bold': activeTab === 'webhooks', 'text-slate-400': activeTab !== 'webhooks' }"
                class="flex flex-col items-center gap-1 px-3 py-1 rounded-xl transition-all">
                <i class="fa-solid fa-satellite-dish text-base"></i>
                <span>Webhooks</span>
            </button>'''

if 'switchTab(\'access\')' not in html or 'Acesso</span>' not in html:
    html = html.replace(mobile_nav_target, mobile_nav_new)

with open('templates/dashboard.html', 'w', encoding='utf-8') as f:
    f.write(html)

print('✅ Update script step 1 concluído com sucesso!')

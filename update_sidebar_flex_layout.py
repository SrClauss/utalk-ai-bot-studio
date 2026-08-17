import re

with open('templates/dashboard.html', 'r', encoding='utf-8') as f:
    html = f.read()

# Replace body style to force no horizontal scroll
html = html.replace(
    "html, body {\n            overflow-x: hidden !important;\n            max-width: 100vw;\n        }",
    "html, body {\n            overflow-x: hidden !important;\n            max-width: 100vw;\n            width: 100%;\n        }"
)

# 1. Update the outer flex wrapper
old_wrapper = '<div x-show="loggedIn" class="flex flex-col min-h-screen w-full" x-transition>'
new_wrapper = '<div x-show="loggedIn" class="flex flex-col md:flex-row min-h-screen w-full max-w-full overflow-x-hidden" x-transition>'

if old_wrapper in html:
    html = html.replace(old_wrapper, new_wrapper)

# 2. Update Aside Sidebar from fixed to sticky top-0 h-screen shrink-0
old_aside = '''        <aside
            class="hidden md:flex flex-col fixed top-0 bottom-0 left-0 z-40 bg-slate-950/95 backdrop-blur-xl border-r border-slate-800/80 transition-all duration-300 select-none shadow-2xl"
            :class="sidebarCollapsed ? 'w-20' : 'w-64'">'''

new_aside = '''        <aside
            class="hidden md:flex flex-col sticky top-0 h-screen shrink-0 z-40 bg-slate-950/95 backdrop-blur-xl border-r border-slate-800/80 transition-all duration-300 select-none shadow-2xl"
            :class="sidebarCollapsed ? 'w-20' : 'w-64'">'''

if old_aside in html:
    html = html.replace(old_aside, new_aside)

# 3. Update Main container to flex-1 min-w-0 without any margin-left or padding-left hacks
old_main = '''        <main class="transition-all duration-300 flex-1 w-full max-w-full overflow-x-hidden px-4 sm:px-6 pt-6 pb-24 md:pb-12 space-y-6"
            :class="sidebarCollapsed ? 'md:pl-20' : 'md:pl-64'">'''

new_main = '''        <main class="transition-all duration-300 flex-1 min-w-0 max-w-full overflow-x-hidden px-4 sm:px-6 pt-6 pb-24 md:pb-12 space-y-6">'''

if old_main in html:
    html = html.replace(old_main, new_main)

with open('templates/dashboard.html', 'w', encoding='utf-8') as f:
    f.write(html)

print('✅ Layout Flexbox aplicado! Sidebar Sticky + Main Flex-1 = Zero sobreposição e Zero Scroll!')

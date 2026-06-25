import os
import re
import sys
import subprocess
import time

base_path = r"c:\Users\Lenovo\Downloads\مشروع لغة برمجة"
engine_path = os.path.join(base_path, "src", "codegen", "express", "static_files", "engine.rs")
theme_path = os.path.join(base_path, "src", "codegen", "express", "theme.rs")
tokens_path = os.path.join(base_path, "src", "codegen", "express", "tokens.rs")

# Read files
with open(engine_path, "r", encoding="utf-8") as f:
    engine_content = f.read()

with open(theme_path, "r", encoding="utf-8") as f:
    theme_content = f.read()

# 1. Extract theme options dynamically from theme.rs
colors = re.findall(r'"(\w+)"\s*=>\s*Some\(', theme_content)
colors = sorted(list(set(colors)))

radii = re.findall(r'"(\w+)"\s*=>\s*radius_\w+', theme_content)
radii = sorted(list(set(radii)))

densities = re.findall(r'"(\w+)"\s*=>\s*density_\w+', theme_content)
densities = sorted(list(set(densities)))

print(f"Extracted Colors: {colors}")
print(f"Extracted Radii: {radii}")
print(f"Extracted Densities: {densities}")

# 2. Extract standard components and their attributes from engine.rs
match_func = re.search(r'fn renderStandardComponent\(.*?\)\s*\{(.*?)\n\}', engine_content, re.DOTALL)
func_body = match_func.group(1) if match_func else engine_content

tag_matches = re.findall(r"tag\s*===\s*'(\w+)'|tag\s*===\s*\"(\w+)\"", func_body)
tags = sorted(list(set([t[0] or t[1] for t in tag_matches if (t[0] or t[1])])))

component_attrs = {}
for tag in tags:
    # Find position of tag in the function body
    pos = func_body.find(f"tag === '{tag}'")
    if pos == -1:
        pos = func_body.find(f'tag === "{tag}"')
    if pos == -1:
        pos = func_body.find(f"'{tag}'")
    
    if pos != -1:
        # Take a block of code and truncate it before the next tag
        block = func_body[pos:pos+3000]
        next_tag_pos = len(block)
        for other_tag in tags:
            if other_tag == tag:
                continue
            other_pos = block.find(f"tag === '{other_tag}'")
            if other_pos == -1:
                other_pos = block.find(f'tag === "{other_tag}"')
            if other_pos != -1 and other_pos < next_tag_pos:
                next_tag_pos = other_pos
        block = block[:next_tag_pos]
        
        # Find getAttr or .find attribute queries
        attrs = re.findall(r"getAttr\(\s*attributes,\s*'([^']+)'", block)
        attrs += re.findall(r"\.find\(\(\[key\]\)\s*=>\s*key\s*===\s*'([^']+)'", block)
        component_attrs[tag] = sorted(list(set(attrs)))
    else:
        component_attrs[tag] = []

print(f"Extracted components and attributes: {component_attrs}")

# Helper to generate test values for attributes
def get_mock_attr_value(attr_name, tag):
    if attr_name == "href" or attr_name == "action_href":
        return '"#"'
    elif attr_name == "label" or attr_name == "text":
        return '"زر الضغط"' if tag == "Button" else '"محتوى نصي تجريبي"'
    elif attr_name == "title":
        return f'"مكون {tag}"'
    elif attr_name == "subtitle" or attr_name == "description":
        return f'"وصف فرعي للمكون {tag} مستخرج تلقائيا"'
    elif attr_name == "eyebrow":
        return '"نبذة علوية"'
    elif attr_name == "badge":
        return '"مميز"'
    elif attr_name == "meta":
        return '"اليوم"'
    elif attr_name == "price":
        return '"199 $"'
    elif attr_name == "action_label":
        return '"تفاعل الآن"'
    elif attr_name == "density":
        return '"comfortable"'
    elif attr_name == "width":
        return '"wide"'
    elif attr_name == "min":
        return '"14rem"'
    elif attr_name == "columns":
        return '"3"'
    elif attr_name == "stretch":
        return 'true'
    elif attr_name == "gap":
        return '"md"'
    elif attr_name == "brand":
        return '"شعار أمانة"'
    elif attr_name == "sticky":
        return 'false'
    elif attr_name == "variant":
        return '"default"'
    elif attr_name == "autoplay":
        return 'false'
    elif attr_name == "height":
        return '"200px"'
    elif attr_name == "effect":
        return '"slide"'
    elif attr_name == "media":
        return '"/assets/luxury_hero.png"'
    elif attr_name == "proof":
        return '"مثبت بالاختبار"'
    elif attr_name == "name":
        return '"input_name"'
    elif attr_name == "type":
        return '"text"'
    elif attr_name == "placeholder":
        return '"اكتب هنا..."'
    elif attr_name == "tone":
        return '"info"'
    elif attr_name == "icon":
        return '"star"'
    elif attr_name == "open":
        return 'modal_open'
    elif attr_name == "closable":
        return 'true'
    elif attr_name == "value":
        return '"99%"'
    elif attr_name == "trend":
        return '"+5%"'
    elif attr_name == "quote":
        return '"تطبيق أمانة هو منصة المستقبل للتطبيقات السحابية الموثوقة."'
    elif attr_name == "author":
        return '"فريق تطوير أمانة"'
    elif attr_name == "role":
        return '"المطور الرئيسي"'
    else:
        return '"قيمة تجريبية"'

# Build Component Test DSL
comp_list_dsl = []
comp_list_dsl.append("            Navbar(brand: \"أمانة للمكونات\", sticky: false):")
comp_list_dsl.append("                a(href: \"#\"): \"الرئيسية\"")
comp_list_dsl.append("                a(href: \"#\"): \"عن المكونات\"")

# Add components in a structured way
for tag, attrs in component_attrs.items():
    if tag in ["Navbar", "Footer", "tab", "panel", "TimelineItem"]:
        # Handled specially
        continue
    
    # Format attributes string
    attr_pairs = []
    for a in attrs:
        val = get_mock_attr_value(a, tag)
        attr_pairs.append(f"{a}: {val}")
    attr_str = ", ".join(attr_pairs)
    paren_str = f"({attr_str})" if attr_str else ""
    
    comp_list_dsl.append(f"            div.section-block(id: \"{tag.lower()}\"):")
    comp_list_dsl.append(f"                h3: \"مكون: {tag}\"")
    
    # Render with children/inner or as leaf
    if tag == "Slides":
        comp_list_dsl.append(f"                Slides{paren_str}:")
        comp_list_dsl.append("                    Card(title: \"الشريحة 1\"):")
        comp_list_dsl.append("                        p: \"محتوى الشريحة الأولى\"")
        comp_list_dsl.append("                    Card(title: \"الشريحة 2\"):")
        comp_list_dsl.append("                        p: \"محتوى الشريحة الثانية\"")
    elif tag == "Tabs":
        comp_list_dsl.append("                Tabs:")
        comp_list_dsl.append("                    tab \"التبويب 1\":")
        comp_list_dsl.append("                        p: \"محتوى التبويب الأول\"")
        comp_list_dsl.append("                    tab \"التبويب 2\":")
        comp_list_dsl.append("                        p: \"محتوى التبويب الثاني\"")
    elif tag == "Accordion":
        comp_list_dsl.append("                Accordion:")
        comp_list_dsl.append("                    panel \"اللوحة 1\":")
        comp_list_dsl.append("                        p: \"تفاصيل اللوحة الأولى\"")
        comp_list_dsl.append("                    panel \"اللوحة 2\":")
        comp_list_dsl.append("                        p: \"تفاصيل اللوحة الثانية\"")
    elif tag == "Timeline":
        comp_list_dsl.append("                Timeline:")
        comp_list_dsl.append("                    TimelineItem(title: \"البداية\", meta: \"2026\"):")
        comp_list_dsl.append("                        p: \"إطلاق منصة أمانة\"")
        comp_list_dsl.append("                    TimelineItem(title: \"الانتشار\", meta: \"2027\"):")
        comp_list_dsl.append("                        p: \"استخدام أمانة في المشاريع الكبرى\"")
    elif tag in ["Grid", "Stack", "Split", "Cluster", "Container", "Section", "Sidebar"]:
        comp_list_dsl.append(f"                {tag}{paren_str}:")
        comp_list_dsl.append("                    Card(title: \"محتوى الحاوية\"):")
        comp_list_dsl.append(f"                        p: \"هذا محتوى تجريبي داخل {tag}\"")
    elif tag == "Modal":
        # Modal needs button to open it
        comp_list_dsl.append("                Button(label: \"فتح النافذة (Modal)\", click: \"modal_open = true\", variant: \"primary\")")
        comp_list_dsl.append(f"                Modal{paren_str}:")
        comp_list_dsl.append("                    p: \"هذا محتوى النافذة المنبثقة.\"")
        comp_list_dsl.append("                    Button(label: \"إغلاق\", click: \"modal_open = false\", variant: \"secondary\")")
    else:
        # Leaf component
        comp_list_dsl.append(f"                {tag}{paren_str}")

# Add Footer at the end
comp_list_dsl.append("            Footer:")
comp_list_dsl.append("                p: \"حقوق النشر محفوظة © 2026 لغة أمانة\"")

components_dsl = f"""app TestComponentsApp:
    title: "مكونات أمانة الشاملة (مستخرجة تلقائياً)"
    db_path: "test_components.db"
    auth_model: User
    capabilities:
        - auth

theme:
    mode: auto
    direction: rtl
    language: ar
    primary: "{colors[0] if colors else 'indigo'}"
    accent: "{colors[1] if len(colors) > 1 else 'cyan'}"

model User:
    email: email unique required
    password: password required min 8

model Lead:
    name: str required
    email: email required
    message: str

route / -> view Home

view Home:
    canvas:
        layout: column
        density: comfortable

    client:
        state modal_open = false

    render:
        div.page:
{chr(10).join(comp_list_dsl)}

            div.section-block(id: "form-test"):
                h3: "نموذج إدخال (Form Block)"
                form [name, email, message]:
                    connect Lead.create
                    submit: "إرسال البيانات"
                    redirect success -> /
"""

# Build Themes Test DSL
themes_dsl = f"""app TestThemesApp:
    title: "اختبار السمات وموضوعات الألوان (مستخرج تلقائياً)"
    db_path: "test_themes.db"
    auth_model: User
    capabilities:
        - auth

theme:
    mode: auto
    direction: rtl
    language: ar
    primary: "{colors[0] if colors else 'indigo'}"
    accent: "{colors[1] if len(colors) > 1 else 'cyan'}"
    radius: {radii[0] if radii else 'soft'}
    density: {densities[0] if densities else 'comfortable'}

model User:
    email: email unique required
    password: password required min 8

route / -> view Home

view Home:
    canvas:
        layout: column
        density: comfortable

    render:
        div.page:
            Container(width: "wide"):
                h1: "Theme Settings Capability Test"
                p: "This page exercises the dynamic theme settings extracted from source code."
                
                Grid(columns: "3"):
                    Card(title: "Extracted Colors"):
                        p: "Colors: {', '.join(colors)}"
                    Card(title: "Extracted Radii"):
                        p: "Radii: {', '.join(radii)}"
                    Card(title: "Extracted Densities"):
                        p: "Densities: {', '.join(densities)}"
                
                div.flex-row(style: "margin-top: 2rem; display: flex; gap: 1rem;"):
                    Button(label: "Primary color themed button", variant: "primary")
                    Button(label: "Secondary color themed button", variant: "secondary")
"""

# Build Grids Test DSL
grids_dsl = """app TestGridsApp:
    title: "مخططات وتوزيعات الشبكة (مستخرج تلقائياً)"
    db_path: "test_grids.db"
    auth_model: User
    capabilities:
        - auth

theme:
    mode: dark
    direction: rtl
    language: ar
    primary: "indigo"
    accent: "cyan"

model User:
    email: email unique required
    password: password required min 8

route / -> view Home

view Home:
    canvas:
        layout: column
        density: comfortable

    render:
        div.page:
            Container(width: "wide"):
                h2: "1. تخطيط الانقسام (Split Layout)"
                Split:
                    div:
                        h3: "الجانب الأيمن"
                        p: "محتوى مقسم بنسبة متوازنة."
                    Card(title: "الجانب الأيسر"):
                        p: "محتوى جانبي مساعد."
                
                h2: "2. الشبكات العادية والتمدد (Grid Stretch)"
                Grid(stretch: true):
                    Card(title: "محتوى قصير"):
                        p: "نص بسيط."
                    Card(title: "محتوى طويل ومتمدد"):
                        p: "نص أطول لاختبار تمدد الارتفاع ليتوافق مع جميع العناصر المجاورة."
                
                h2: "3. التجميع (Cluster Layout)"
                Cluster:
                    Button(label: "عنصر 1")
                    Button(label: "عنصر 2")
                    Button(label: "عنصر 3")
                
                h2: "4. عمود جانبي (Sidebar Layout)"
                Sidebar:
                    h3: "محتوى الشريط الجانبي"
                    p: "تفاصيل جانبية هامة."
"""

# Build CSS Test DSL
css_dsl = """app TestCssApp:
    title: "اختبار الـ CSS والخصائص المنطقية (مستخرج تلقائياً)"
    db_path: "test_css.db"
    auth_model: User
    capabilities:
        - auth

theme:
    mode: auto
    direction: rtl
    language: ar
    primary: "indigo"
    accent: "cyan"

variant Card.custom_glow:
    base:
        background: "rgba(10, 10, 12, 0.9)"
        border: "1px solid #d4af37"
        border-radius: "20px"
        padding: "2rem"
    hover:
        border-color: "#ffffff"
        transform: "translateY(-4px)"

model User:
    email: email unique required
    password: password required min 8

route / -> view Home

view Home:
    canvas:
        layout: column
        density: comfortable

    render:
        div.page:
            Container(width: "wide"):
                h1: "CSS & Visual Design Engine Test"
                Grid:
                    Card(title: "بطاقة مخصصة التوهج", variant: "custom_glow"):
                        p: "هذه البطاقة تستخدم شكلاً مخصصاً تم تعريفه عبر DSL بحدود ذهبية وتوهج داكن."
                
                h2: "اختبار الخصائص المنطقية (Logical Properties)"
                Grid:
                    div(style: "margin-inline-start: 2rem; border-inline-start: 4px solid var(--color-primary); padding-inline: 1.5rem;"):
                        Card(title: "إزاحة البداية (Start Offset)"):
                            p: "حد جانبي منطقي يبدأ من اليمين في لغة الضاد."
                    
                    div(style: "margin-inline-end: 2rem; border-inline-end: 4px solid var(--color-accent); padding-inline: 1.5rem;"):
                        Card(title: "إزاحة النهاية (End Offset)"):
                            p: "حد جانبي منطقي ينتهي في اليسار في لغة الضاد."
"""

# Write the generated DSL files to examples/
os.makedirs(os.path.join(base_path, "examples"), exist_ok=True)
with open(os.path.join(base_path, "examples", "test_components.amana"), "w", encoding="utf-8") as f:
    f.write(components_dsl)

with open(os.path.join(base_path, "examples", "test_themes.amana"), "w", encoding="utf-8") as f:
    f.write(themes_dsl)

with open(os.path.join(base_path, "examples", "test_grids.amana"), "w", encoding="utf-8") as f:
    f.write(grids_dsl)

with open(os.path.join(base_path, "examples", "test_css.amana"), "w", encoding="utf-8") as f:
    f.write(css_dsl)

print("Programmatic extraction and DSL generation complete!")

# ----------------------------------------------------
# Compilation and Server Launch Section
# ----------------------------------------------------
apps = [
    {"name": "components", "port": 3001, "src": "examples/test_components.amana", "dest": "test_components_dist"},
    {"name": "themes", "port": 3002, "src": "examples/test_themes.amana", "dest": "test_themes_dist"},
    {"name": "grids", "port": 3003, "src": "examples/test_grids.amana", "dest": "test_grids_dist"},
    {"name": "css", "port": 3004, "src": "examples/test_css.amana", "dest": "test_css_dist"},
]

def kill_process_on_port(port):
    try:
        out = subprocess.check_output(f'netstat -ano | findstr :{port}', shell=True).decode()
        pids = set()
        for line in out.strip().split('\n'):
            parts = line.strip().split()
            if len(parts) >= 5:
                pid = parts[-1]
                if pid.isdigit() and int(pid) > 0:
                    pids.add(pid)
        for pid in pids:
            print(f"Killing process {pid} on port {port}...")
            subprocess.run(f'taskkill /F /PID {pid}', shell=True)
    except subprocess.CalledProcessError:
        pass

# Build and start each application
for app in apps:
    print(f"\n--- Processing App: {app['name']} (Port: {app['port']}) ---")
    
    # 1. Kill any process occupying the port
    kill_process_on_port(app["port"])
    
    # 2. Compile Amana app
    print(f"Compiling {app['src']} to {app['dest']}...")
    build_cmd = f"cargo run -- build {app['src']} {app['dest']}"
    res = subprocess.run(build_cmd, shell=True, cwd=base_path)
    if res.returncode != 0:
        print(f"FAILED to compile {app['name']}")
        continue
    
    # 3. Ensure node_modules exists, otherwise run npm install
    dest_abs = os.path.join(base_path, app["dest"])
    nm_path = os.path.join(dest_abs, "node_modules")
    if not os.path.exists(nm_path):
        print(f"node_modules not found in {app['dest']}. Running npm install...")
        npm_install_res = subprocess.run("npm install", shell=True, cwd=dest_abs)
        if npm_install_res.returncode != 0:
            print(f"FAILED to run npm install for {app['name']}")
            continue
            
    # 4. Launch Node Server on the specified PORT
    print(f"Launching Express server on port {app['port']}...")
    env = os.environ.copy()
    env["PORT"] = str(app["port"])
    
    log_path = os.path.join(base_path, f"server_{app['name']}.log")
    
    # Launch in background decoupled from python process using native Windows start/B redirection
    # To handle spaces in path, we wrap log_path in double quotes.
    cmd = f'start /B node app.js > "{log_path}" 2>&1'
    proc = subprocess.Popen(
        cmd,
        env=env,
        cwd=dest_abs,
        shell=True
    )
    print(f"Server {app['name']} launched successfully (PID: {proc.pid})!")

print("\nAll test servers generated, compiled and launched in the background successfully!")

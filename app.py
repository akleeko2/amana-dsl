from pathlib import Path

code = r'''app ProLanding:
    title: "Amana Pro Landing"
    db_path: "pro_landing.db"
    auth_model: User
    capabilities:
        - auth
        - api.rest

model User:
    name: str
    email: email unique
    password: password
    role: str default "visitor"
    bio: str
    is_active: bool default true
    created_at: datetime default "CURRENT_TIMESTAMP"

model Feature:
    title: str
    description: str
    order: int default 0
    is_highlighted: bool default false

model Testimonial:
    author_name: str
    author_role: str
    content: str
    rating: int default 5
    is_featured: bool default false

model PricingPlan:
    name: str
    price: money
    interval: str default "month"
    description: str
    features: str
    is_popular: bool default false
    cta_text: str default "ابدأ الآن"
    sort_order: int default 0

model Subscriber:
    email: email unique
    name: str
    source: str default "landing"
    subscribed_at: datetime default "CURRENT_TIMESTAMP"

model ContactMessage:
    name: str
    email: email
    subject: str
    message: str
    status: str default "unread"
    created_at: datetime default "CURRENT_TIMESTAMP"

model Project:
    name: str
    description: str
    status: str default "active"
    owner_id: int foreign_key User(id) on_delete CASCADE
    created_at: datetime default "CURRENT_TIMESTAMP"

model Task:
    title: str
    status: str default "pending"
    project_id: int foreign_key Project(id) on_delete CASCADE
    owner_id: int foreign_key User(id) on_delete CASCADE

route / -> view HeroLanding
route /features -> view FeaturesPage
route /pricing -> view PricingPage
route /testimonials -> view TestimonialsPage
route /login -> view LoginPage
route /register -> view RegisterPage
route /dashboard -> view DashboardPage
route /contact -> view ContactPage

component NavBar:
    render:
        nav.navbar:
            div.nav-wrap:
                a.brand(href: "/"):
                    span.brand-mark: "أ"
                    span.brand-name: "أمانة"
                div.nav-links:
                    a(href: "/features"): "المميزات"
                    a(href: "/pricing"): "التسعير"
                    a(href: "/testimonials"): "آراء العملاء"
                    a(href: "/contact"): "تواصل"
                div.nav-actions:
                    a.nav-login(href: "/login"): "دخول"
                    a.nav-cta(href: "/register"): "ابدأ الآن"
    style:
        .navbar:
            position: sticky
            top: 0
            z-index: 50
            background: #0b1220
            border-bottom: 1px solid #1f2d45

        .nav-wrap:
            max-width: 1180px
            margin: 0 auto
            padding: 18px 28px
            layout: row
            align-items: center
            justify-content: space-between
            gap: lg

        .brand:
            layout: row
            align-items: center
            gap: sm
            color: #eef3ff
            text-decoration: none

        .brand-mark:
            width: 38px
            height: 38px
            radius: full
            background: #6366f1
            color: #ffffff
            layout: center
            font-weight: 900

        .brand-name:
            font: heading
            font-size: 22px
            weight: 900

        .nav-links:
            layout: row
            gap: lg

        .nav-links a:
            color: #aeb9d4
            text-decoration: none
            weight: 700
            transition: smooth

        .nav-actions:
            layout: row
            gap: sm

        .nav-login:
            color: #eef3ff
            text-decoration: none
            padding: 12px 20px
            radius: full
            border: 1px solid #24324a
            weight: 800

        .nav-cta:
            background: #6366f1
            color: #ffffff
            text-decoration: none
            padding: 12px 22px
            radius: full
            weight: 900
            shadow: smooth

component Footer:
    render:
        footer.site-footer:
            div.footer-wrap:
                div.footer-brand:
                    h3: "أمانة"
                    p: "لغة DSL لبناء تطبيقات ويب كاملة من ملف واحد، مع تحقق دلالي وIR آمن وقابل للتوسع."
                div.footer-links:
                    a(href: "/features"): "المميزات"
                    a(href: "/pricing"): "التسعير"
                    a(href: "/contact"): "تواصل"
                    a(href: "/dashboard"): "لوحة التحكم"
            div.footer-bottom:
                p: "© 2025 Amana. مبني للمطورين الذين يحبون الوضوح."
    style:
        .site-footer:
            background: #08101e
            border-top: 1px solid #1f2d45
            color: #aeb9d4
            padding: 48px 28px 28px

        .footer-wrap:
            max-width: 1180px
            margin: 0 auto
            layout: row
            justify-content: space-between
            gap: 2xl

        .footer-brand:
            max-width: 520px
            layout: stack
            gap: sm

        .footer-brand h3:
            color: #eef3ff
            font: heading
            font-size: 28px
            weight: 900
            margin: 0

        .footer-brand p:
            margin: 0
            leading: 1.8

        .footer-links:
            layout: row
            gap: lg

        .footer-links a:
            color: #aeb9d4
            text-decoration: none
            weight: 700

        .footer-bottom:
            max-width: 1180px
            margin: 32px auto 0
            padding-top: 22px
            border-top: 1px solid #1f2d45
            text-align: center
            color: #6f7f9d

view HeroLanding:
    render:
        div.page(dir: "rtl"):
            NavBar()
            section.hero:
                div.hero-inner:
                    div.hero-copy:
                        p.kicker: "لغة تطبيقات ويب كاملة"
                        h1.hero-title: "ابن تطبيقك الكامل من ملف واحد، بوضوح وأمان."
                        p.hero-lead: "أمانة تجمع النماذج، المسارات، الواجهات، النماذج التفاعلية، قواعد الحماية، والتصميم داخل لغة DSL نظيفة تولد تطبيق Node.js جاهزا للتشغيل."
                        div.hero-actions:
                            a.btn-main(href: "/register"): "ابدأ الآن"
                            a.btn-soft(href: "/features"): "استكشف المميزات"
                        div.trust-row:
                            span.trust-pill: "Semantic Validation"
                            span.trust-pill: "Amana IR"
                            span.trust-pill: "Secure Forms"

                    div.hero-card:
                        div.card-head:
                            p.kicker: "Amana Pipeline"
                            h2: "Source → IR → Runtime"
                            p: "الكود لا يتحول إلى نصوص عشوائية. يمر أولا عبر parser وsemantic validation ثم IR قبل التوليد."
                        div.pipeline:
                            div.pipe:
                                span.pipe-no: "01"
                                h3: "Parser"
                                p: "يفهم بنية اللغة."
                            div.pipe:
                                span.pipe-no: "02"
                                h3: "Semantic"
                                p: "يرفض الأخطاء مبكرا."
                            div.pipe:
                                span.pipe-no: "03"
                                h3: "Runtime"
                                p: "يشغل التطبيق بأمان."
                        div.secure-line:
                            code: "default owner_id = User.current.id"
                            span: "ملكية آمنة من السيرفر"

            section.features:
                div.section-head:
                    p.kicker: "لماذا أمانة؟"
                    h2.section-title: "ليست مجرد مولد صفحات، بل لغة لها قلب حقيقي."
                    p.section-lead: "كل شيء يمر عبر AST وSemantic Validation وIR قبل التوليد، لذلك يمكن أن تكبر أمانة لاحقا إلى targets متعددة بدون إعادة اختراع اللغة."
                div.feature-grid:
                    div.feature-card:
                        span.feature-no: "01"
                        h3: "وضوح كلاسيكي"
                        p: "صياغة مبنية على الإزاحة تجعل الصفحة والمسارات والنماذج قابلة للقراءة بسرعة."
                    div.feature-card:
                        span.feature-no: "02"
                        h3: "أمان من اللغة"
                        p: "النماذج تدعم defaults وwhere على السيرفر، فلا تعتمد الملكية على hidden inputs."
                    div.feature-card:
                        span.feature-no: "03"
                        h3: "قابلية التوسع"
                        p: "Express هو backend حالي فقط، أما القلب الحقيقي فهو IR يمكن أن يغذي backends أخرى."
                    div.feature-card:
                        span.feature-no: "04"
                        h3: "تصميم موحد"
                        p: "CSS DSL يمنحك tokens وlayouts واضحة بدلا من فوضى CSS المتكررة."
            section.newsletter:
                div.newsletter-box:
                    div:
                        p.kicker: "ابق على اتصال"
                        h2: "احصل على تحديثات أمانة."
                        p: "اكتب بريدك وسيتم تسجيله في قاعدة البيانات عبر form آمن."
                    form [email]:
                        connect Subscriber.create
                        default source = "landing"
                        redirect success -> /
            Footer()
    style:
        .page:
            background: #0b1220
            color: #eef3ff
            min-height: 100vh
            font-family: system-ui

        .hero:
            padding: 58px 28px 34px

        .hero-inner:
            max-width: 1180px
            margin: 0 auto
            min-height: 620px
            layout: grid
            columns: 2
            gap: 2xl
            align-items: center

        .hero-copy:
            layout: stack
            gap: lg

        .kicker:
            color: #7c83ff
            size: sm
            weight: 900
            tracking: 0.1em
            margin: 0

        .hero-title:
            font: heading
            font-size: 54px
            weight: 900
            leading: 1.18
            margin: 0

        .hero-lead:
            color: #c7d2e6
            font-size: 20px
            leading: 1.9
            margin: 0

        .hero-actions:
            layout: row
            gap: md
            margin-top: md

        .btn-main:
            background: #6366f1
            color: #ffffff
            padding: 15px 30px
            radius: full
            text-decoration: none
            weight: 900
            shadow: smooth

        .btn-soft:
            background: #111a2c
            color: #eef3ff
            padding: 15px 30px
            radius: full
            border: 1px solid #24324a
            text-decoration: none
            weight: 900

        .trust-row:
            layout: row
            gap: sm
            margin-top: md

        .trust-pill:
            background: #111a2c
            color: #aeb9d4
            padding: 10px 14px
            radius: full
            border: 1px solid #24324a
            size: sm
            weight: 800

        .hero-card:
            background: #111a2c
            border: 1px solid #263650
            radius: large
            padding: 28px
            shadow: large
            layout: stack
            gap: lg

        .card-head:
            background: #0d1628
            border: 1px solid #22304a
            radius: large
            padding: xl
            layout: stack
            gap: sm

        .card-head h2:
            font: heading
            font-size: 34px
            weight: 900
            margin: 0

        .card-head p:
            color: #c7d2e6
            leading: 1.8
            margin: 0

        .pipeline:
            layout: grid
            columns: 3
            gap: md

        .pipe:
            background: #0d1628
            border: 1px solid #22304a
            radius: large
            padding: lg
            layout: stack
            gap: sm
            min-height: 150px

        .pipe-no:
            color: #7c83ff
            weight: 900
            font-size: 22px

        .pipe h3:
            font: heading
            font-size: 20px
            weight: 900
            margin: 0

        .pipe p:
            color: #aeb9d4
            leading: 1.6
            margin: 0

        .secure-line:
            background: #182338
            border: 1px solid #2c3e5d
            radius: large
            padding: lg
            layout: row
            gap: lg
            align-items: center
            justify-content: space-between

        .secure-line code:
            color: #ffffff
            font: mono

        .secure-line span:
            color: #c7d2e6
            weight: 900

        .features:
            padding: 34px 28px 68px

        .section-head:
            max-width: 1180px
            margin: 0 auto
            background: #111a2c
            border: 1px solid #24324a
            radius: large
            padding: 42px
            layout: stack
            gap: md

        .section-title:
            font: heading
            font-size: 42px
            weight: 900
            leading: 1.24
            margin: 0

        .section-lead:
            color: #c7d2e6
            font-size: 19px
            leading: 1.9
            margin: 0

        .feature-grid:
            max-width: 1180px
            margin: 22px auto 0
            layout: grid
            columns: 4
            gap: lg

        .feature-card:
            background: #0d1628
            border: 1px solid #22304a
            radius: large
            padding: xl
            shadow: smooth
            layout: stack
            gap: md
            min-height: 250px

        .feature-no:
            color: #7c83ff
            font-size: 28px
            weight: 900

        .feature-card h3:
            font: heading
            font-size: 26px
            weight: 900
            margin: 0

        .feature-card p:
            color: #c7d2e6
            leading: 1.8
            margin: 0

        .newsletter:
            padding: 0 28px 68px

        .newsletter-box:
            max-width: 1180px
            margin: 0 auto
            background: #101a2d
            border: 1px solid #24324a
            radius: large
            padding: 34px
            layout: grid
            columns: 2
            gap: 2xl
            align-items: center

        .newsletter-box h2:
            font: heading
            font-size: 32px
            weight: 900
            margin: 0

        .newsletter-box p:
            color: #c7d2e6
            leading: 1.8

view FeaturesPage:
    render:
        div.page(dir: "rtl"):
            NavBar()
            section.simple-hero:
                p.kicker: "المميزات"
                h1: "قدرات عملية لبناء تطبيقات أسرع."
                p: "هذه الصفحة تعرض المزايا الأساسية للغة أمانة بطريقة مباشرة."
            section.feature-grid-page:
                div.feature-card:
                    span.feature-no: "01"
                    h3: "تحقق دلالي"
                    p: "يرفض الأخطاء قبل توليد التطبيق."
                div.feature-card:
                    span.feature-no: "02"
                    h3: "IR مستقل"
                    p: "يفتح الباب لعدة backends مستقبلا."
                div.feature-card:
                    span.feature-no: "03"
                    h3: "نماذج آمنة"
                    p: "CSRF وdefaults وwhere على السيرفر."
                div.feature-card:
                    span.feature-no: "04"
                    h3: "CSS DSL"
                    p: "تصميم سريع ومنظم عبر tokens."
            Footer()
    style:
        .page:
            background: #0b1220
            color: #eef3ff
            min-height: 100vh

        .simple-hero:
            max-width: 900px
            margin: 0 auto
            padding: 96px 28px 36px
            text-align: center
            layout: stack
            gap: md

        .simple-hero h1:
            font: heading
            font-size: 46px
            weight: 900
            margin: 0

        .simple-hero p:
            color: #c7d2e6
            font-size: 19px
            leading: 1.8

        .kicker:
            color: #7c83ff
            weight: 900

        .feature-grid-page:
            max-width: 1180px
            margin: 0 auto
            padding: 20px 28px 80px
            layout: grid
            columns: 4
            gap: lg

        .feature-card:
            background: #111a2c
            border: 1px solid #24324a
            radius: large
            padding: xl
            layout: stack
            gap: md
            min-height: 240px

        .feature-no:
            color: #7c83ff
            font-size: 28px
            weight: 900

        .feature-card h3:
            font: heading
            font-size: 24px
            weight: 900
            margin: 0

        .feature-card p:
            color: #c7d2e6
            leading: 1.8
            margin: 0

view PricingPage:
    server:
        fetch plans = PricingPlan.all()
    render:
        div.page(dir: "rtl"):
            NavBar()
            section.simple-hero:
                p.kicker: "التسعير"
                h1: "خطط بسيطة وواضحة."
                p: "يمكنك ملء جدول PricingPlan من قاعدة البيانات، وستظهر الخطط هنا."
            section.pricing-grid:
                for plan in plans:
                    div.pricing-card:
                        if plan.is_popular:
                            span.popular: "الأكثر اختيارا"
                        h2: plan.name
                        p.plan-price: plan.price
                        p: plan.description
                        p.plan-features: plan.features
                        a.btn-main(href: "/register"): plan.cta_text
            Footer()
    style:
        .page:
            background: #0b1220
            color: #eef3ff
            min-height: 100vh

        .simple-hero:
            max-width: 900px
            margin: 0 auto
            padding: 96px 28px 36px
            text-align: center
            layout: stack
            gap: md

        .simple-hero h1:
            font-size: 46px
            weight: 900
            margin: 0

        .simple-hero p:
            color: #c7d2e6
            leading: 1.8

        .kicker:
            color: #7c83ff
            weight: 900

        .pricing-grid:
            max-width: 1180px
            margin: 0 auto
            padding: 20px 28px 80px
            layout: grid
            columns: 3
            gap: lg

        .pricing-card:
            background: #111a2c
            border: 1px solid #24324a
            radius: large
            padding: xl
            layout: stack
            gap: md
            shadow: smooth

        .popular:
            background: #6366f1
            color: #ffffff
            padding: 8px 12px
            radius: full
            width: fit
            weight: 900

        .plan-price:
            color: #ffffff
            font-size: 34px
            weight: 900

        .plan-features:
            color: #c7d2e6
            leading: 1.8

        .btn-main:
            background: #6366f1
            color: #ffffff
            padding: 14px 24px
            radius: full
            text-decoration: none
            text-align: center
            weight: 900

view TestimonialsPage:
    server:
        fetch all_testimonials = Testimonial.all()
    render:
        div.page(dir: "rtl"):
            NavBar()
            section.simple-hero:
                p.kicker: "آراء العملاء"
                h1: "ما يقوله المستخدمون."
                p: "عند إضافة شهادات في قاعدة البيانات ستظهر في هذه الصفحة."
            section.testimonial-grid:
                for t in all_testimonials:
                    div.testimonial-card:
                        p.rating: t.rating
                        p.content: t.content
                        h3: t.author_name
                        span: t.author_role
            Footer()
    style:
        .page:
            background: #0b1220
            color: #eef3ff
            min-height: 100vh

        .simple-hero:
            max-width: 900px
            margin: 0 auto
            padding: 96px 28px 36px
            text-align: center
            layout: stack
            gap: md

        .simple-hero h1:
            font-size: 46px
            weight: 900
            margin: 0

        .simple-hero p:
            color: #c7d2e6
            leading: 1.8

        .kicker:
            color: #7c83ff
            weight: 900

        .testimonial-grid:
            max-width: 1180px
            margin: 0 auto
            padding: 20px 28px 80px
            layout: grid
            columns: 3
            gap: lg

        .testimonial-card:
            background: #111a2c
            border: 1px solid #24324a
            radius: large
            padding: xl
            layout: stack
            gap: md

        .rating:
            color: #7c83ff
            weight: 900

        .content:
            color: #c7d2e6
            leading: 1.8

view LoginPage:
    client:
        state email = ""
        state password = ""
    render:
        div.page(dir: "rtl"):
            NavBar()
            section.auth:
                div.auth-card:
                    p.kicker: "دخول"
                    h1: "مرحبا بعودتك"
                    p: "سجل الدخول للوصول إلى لوحة التحكم."
                    form [email, password]:
                        connect User.login
                        redirect success -> /dashboard
                    p.auth-note: "لا تملك حسابا؟"
                    a.btn-soft(href: "/register"): "إنشاء حساب"
            Footer()
    style:
        .page:
            background: #0b1220
            color: #eef3ff
            min-height: 100vh

        .auth:
            min-height: 620px
            layout: center
            padding: 64px 28px

        .auth-card:
            width: 100%
            max-width: 460px
            background: #111a2c
            border: 1px solid #24324a
            radius: large
            padding: 2xl
            layout: stack
            gap: md
            shadow: large

        .auth-card h1:
            font-size: 36px
            weight: 900
            margin: 0

        .auth-card p:
            color: #c7d2e6
            leading: 1.8

        .kicker:
            color: #7c83ff
            weight: 900

        .btn-soft:
            background: #0d1628
            color: #eef3ff
            padding: 14px 22px
            radius: full
            border: 1px solid #24324a
            text-decoration: none
            text-align: center
            weight: 900

view RegisterPage:
    client:
        state name = ""
        state email = ""
        state password = ""
    render:
        div.page(dir: "rtl"):
            NavBar()
            section.auth:
                div.auth-card:
                    p.kicker: "تسجيل"
                    h1: "أنشئ حسابك"
                    p: "ابدأ ببناء تطبيقك الأول في دقائق."
                    form [name, email, password]:
                        connect User.create
                        default role = "visitor"
                        redirect success -> /dashboard
                    p.auth-note: "لديك حساب بالفعل؟"
                    a.btn-soft(href: "/login"): "تسجيل الدخول"
            Footer()
    style:
        .page:
            background: #0b1220
            color: #eef3ff
            min-height: 100vh

        .auth:
            min-height: 620px
            layout: center
            padding: 64px 28px

        .auth-card:
            width: 100%
            max-width: 460px
            background: #111a2c
            border: 1px solid #24324a
            radius: large
            padding: 2xl
            layout: stack
            gap: md
            shadow: large

        .auth-card h1:
            font-size: 36px
            weight: 900
            margin: 0

        .auth-card p:
            color: #c7d2e6
            leading: 1.8

        .kicker:
            color: #7c83ff
            weight: 900

        .btn-soft:
            background: #0d1628
            color: #eef3ff
            padding: 14px 22px
            radius: full
            border: 1px solid #24324a
            text-decoration: none
            text-align: center
            weight: 900

view DashboardPage:
    protected:
        allow: User.current != null
        deny: -> /login
        unauthenticated: -> /login
    server:
        fetch my_projects = Project.filter(owner_id: User.current.id)
        fetch my_tasks = Task.filter(owner_id: User.current.id)
        fetch total_subscribers = Subscriber.count()
    render:
        div.page(dir: "rtl"):
            NavBar()
            section.dashboard:
                div.dashboard-head:
                    p.kicker: "لوحة التحكم"
                    h1: "مرحبا"
                    p: User.current.name
                div.stats-grid:
                    div.stat-card:
                        span.stat-value: my_projects.length
                        p: "مشاريع"
                    div.stat-card:
                        span.stat-value: my_tasks.length
                        p: "مهام"
                    div.stat-card:
                        span.stat-value: total_subscribers
                        p: "مشتركين"
                div.dashboard-grid:
                    div.panel:
                        h2: "مشروع جديد"
                        form [name, description]:
                            connect Project.create
                            default owner_id = User.current.id
                            default status = "active"
                            redirect success -> /dashboard
                    div.panel:
                        h2: "مشاريعي"
                        if my_projects.length == 0:
                            p.empty: "لا توجد مشاريع بعد."
                        else:
                            for p in my_projects:
                                div.project-row:
                                    h3: p.name
                                    p: p.description
                                    form [id, name, description, status]:
                                        connect Project.update
                                        where owner_id = User.current.id
                                        redirect success -> /dashboard
                                    form [id]:
                                        connect Project.delete
                                        where owner_id = User.current.id
                                        redirect success -> /dashboard
            Footer()
    style:
        .page:
            background: #0b1220
            color: #eef3ff
            min-height: 100vh

        .dashboard:
            max-width: 1180px
            margin: 0 auto
            padding: 72px 28px
            layout: stack
            gap: xl

        .dashboard-head:
            background: #111a2c
            border: 1px solid #24324a
            radius: large
            padding: 2xl
            layout: stack
            gap: sm

        .dashboard-head h1:
            font-size: 42px
            weight: 900
            margin: 0

        .dashboard-head p:
            color: #c7d2e6

        .kicker:
            color: #7c83ff
            weight: 900

        .stats-grid:
            layout: grid
            columns: 3
            gap: lg

        .stat-card:
            background: #111a2c
            border: 1px solid #24324a
            radius: large
            padding: xl
            layout: stack
            gap: sm

        .stat-value:
            font-size: 38px
            weight: 900
            color: #ffffff

        .stat-card p:
            color: #c7d2e6
            margin: 0

        .dashboard-grid:
            layout: grid
            columns: 2
            gap: lg

        .panel:
            background: #111a2c
            border: 1px solid #24324a
            radius: large
            padding: xl
            layout: stack
            gap: lg

        .panel h2:
            font-size: 28px
            weight: 900
            margin: 0

        .project-row:
            background: #0d1628
            border: 1px solid #22304a
            radius: large
            padding: lg
            layout: stack
            gap: sm

        .project-row h3:
            margin: 0
            weight: 900

        .project-row p:
            color: #c7d2e6
            margin: 0

        .empty:
            color: #aeb9d4

view ContactPage:
    client:
        state name = ""
        state email = ""
        state subject = ""
        state message = ""
    render:
        div.page(dir: "rtl"):
            NavBar()
            section.contact:
                div.contact-info:
                    p.kicker: "تواصل معنا"
                    h1: "نحب أن نسمع منك."
                    p: "راسلنا بسؤالك أو اقتراحك، وسيتم حفظ الرسالة في قاعدة البيانات."
                div.contact-card:
                    form [name, email, subject, message]:
                        connect ContactMessage.create
                        redirect success -> /contact
            Footer()
    style:
        .page:
            background: #0b1220
            color: #eef3ff
            min-height: 100vh

        .contact:
            max-width: 1180px
            margin: 0 auto
            padding: 72px 28px
            layout: grid
            columns: 2
            gap: 2xl
            align-items: center

        .contact-info:
            layout: stack
            gap: md

        .contact-info h1:
            font-size: 46px
            weight: 900
            margin: 0

        .contact-info p:
            color: #c7d2e6
            leading: 1.8

        .kicker:
            color: #7c83ff
            weight: 900

        .contact-card:
            background: #111a2c
            border: 1px solid #24324a
            radius: large
            padding: 2xl
            shadow: large
'''
path = Path("./pro-landing-fixed.amana")
path.write_text(code, encoding="utf-8")
print(f"Created {path} ({path.stat().st_size} bytes)")
// src/codegen/express/tokens.rs
#![allow(dead_code)]
//
// طبقات رموز التصميم الثلاث لمحرّك أمانة
// Layer 1: Primitives (قيم خام — ألوان، مسافات، أنصاف أقطار، ظلال)
// Layer 2: Semantic  (رموز دلالية — أسطح، نصوص، حدود)
// Layer 3: Component (رموز مكوّنات — أزرار، بطاقات، نوافذ)

// ─── Layer 1: Color Primitives ───────────────────────────────────────────────

pub struct ColorScale {
    pub base:  &'static str,  // اللون الأساسي  
    pub soft:  &'static str,  // خلفية ناعمة (10-16%)
    pub dark:  &'static str,  // نسخة داكنة (70-80%)
    pub light: &'static str,  // نسخة فاتحة جداً (2-5%)
    pub mid:   &'static str,  // منتصف (40-60%)
}

pub const INDIGO: ColorScale = ColorScale {
    base:  "#4f46e5", soft: "#eef2ff", dark: "#312e81",
    light: "#faf5ff", mid: "#6366f1",
};

pub const EMERALD: ColorScale = ColorScale {
    base:  "#059669", soft: "#ecfdf5", dark: "#064e3b",
    light: "#f0fdf4", mid: "#10b981",
};

pub const ZINC: ColorScale = ColorScale {
    base:  "#334155", soft: "#f1f5f9", dark: "#0f172a",
    light: "#f8fafc", mid: "#64748b",
};

pub const ROSE: ColorScale = ColorScale {
    base:  "#e11d48", soft: "#fff1f2", dark: "#881337",
    light: "#fef2f2", mid: "#f43f5e",
};

pub const CYAN: ColorScale = ColorScale {
    base:  "#06b6d4", soft: "#ecfeff", dark: "#164e63",
    light: "#f0fdff", mid: "#22d3ee",
};

pub const VIOLET: ColorScale = ColorScale {
    base:  "#7c3aed", soft: "#f5f3ff", dark: "#4c1d95",
    light: "#faf5ff", mid: "#8b5cf6",
};

// ─── Layer 1: Spacing Primitives ─────────────────────────────────────────────

pub struct SpacingScale {
    pub xs: &'static str,   // 0.25rem
    pub sm: &'static str,   // 0.5rem
    pub md: &'static str,   // 1rem
    pub lg: &'static str,   // 1.5rem
    pub xl: &'static str,   // 2rem
    pub xl2: &'static str,  // 3rem
    pub xl3: &'static str,  // 4.5rem
    pub xl4: &'static str,  // 6rem
}

pub const SPACING: SpacingScale = SpacingScale {
    xs: "0.25rem",
    sm: "0.5rem",
    md: "1rem",
    lg: "1.5rem",
    xl: "2rem",
    xl2: "3rem",
    xl3: "4.5rem",
    xl4: "6rem",
};

// ─── Layer 1: Radius Primitives ──────────────────────────────────────────────

pub struct RadiusScale {
    pub none: &'static str,
    pub sharp: &'static str,
    pub soft: &'static str,
    pub xl: &'static str,
    pub xl2: &'static str,
    pub round: &'static str,
    pub pill: &'static str,
}

pub const RADIUS: RadiusScale = RadiusScale {
    none: "0",
    sharp: "4px",
    soft: "10px",
    xl: "18px",
    xl2: "24px",
    round: "50%",
    pill: "999px",
};

// ─── Layer 1: Shadow Primitives ──────────────────────────────────────────────

pub struct ShadowScale {
    pub none: &'static str,
    pub soft: &'static str,
    pub smooth: &'static str,
    pub floating: &'static str,
    pub strong: &'static str,
}

pub const SHADOW: ShadowScale = ShadowScale {
    none: "none",
    soft: "0 2px 8px -2px rgba(2,6,23,0.18), 0 1px 2px rgba(2,6,23,0.08)",
    smooth: "0 4px 16px -4px rgba(2,6,23,0.22), 0 2px 4px rgba(2,6,23,0.1)",
    floating: "0 12px 32px -8px rgba(2,6,23,0.32), 0 4px 8px rgba(2,6,23,0.12)",
    strong: "0 24px 52px -12px rgba(2,6,23,0.48), 0 8px 16px rgba(2,6,23,0.2)",
};

// ─── Layer 2: Surface Semantic Tokens ────────────────────────────────────────

pub struct SurfaceTokens {
    /// الخلفية الرئيسية للصفحة
    pub base: &'static str,
    /// خلفية الحاوية الرئيسية
    pub canvas: &'static str,
    /// خلفية فاتحة للتمييز
    pub muted: &'static str,
    /// خلفية عناصر مرتفعة (بطاقات، لوحات)
    pub elevated: &'static str,
    /// خلفية زجاجية (glassmorphism)
    pub glass_bg: &'static str,
    /// حد زجاجي
    pub glass_border: &'static str,
}

pub const SURFACE_DARK: SurfaceTokens = SurfaceTokens {
    base: "#0b1020",
    canvas: "#050816",
    muted: "#111827",
    elevated: "#151d31",
    glass_bg: "rgba(15, 23, 42, 0.55)",
    glass_border: "rgba(255, 255, 255, 0.12)",
};

pub const SURFACE_LIGHT: SurfaceTokens = SurfaceTokens {
    base: "#ffffff",
    canvas: "#f8fafc",
    muted: "#f8fafc",
    elevated: "#ffffff",
    glass_bg: "rgba(255, 255, 255, 0.68)",
    glass_border: "rgba(15, 23, 42, 0.1)",
};

// ─── Layer 2: Border Semantic Tokens ─────────────────────────────────────────

pub struct BorderTokens {
    /// حد خفيف (فواصل، بطاقات)
    pub subtle: &'static str,
    /// حد قوي (تمييز، تحديد نشط)
    pub strong: &'static str,
    /// حد التركيز (accessibility)
    pub focus: &'static str,
}

pub const BORDER_DARK: BorderTokens = BorderTokens {
    subtle: "rgba(148,163,184,0.18)",
    strong: "rgba(148,163,184,0.45)",
    focus: "#818cf8",
};

pub const BORDER_LIGHT: BorderTokens = BorderTokens {
    subtle: "rgba(15,23,42,0.12)",
    strong: "rgba(15,23,42,0.32)",
    focus: "#4f46e5",
};

// ─── Layer 2: Text Semantic Tokens ───────────────────────────────────────────

pub struct TextTokens {
    pub primary: &'static str,
    pub secondary: &'static str,
    pub muted: &'static str,
}

pub const TEXT_DARK: TextTokens = TextTokens {
    primary: "#f8fafc",
    secondary: "#cbd5e1",
    muted: "#64748b",
};

pub const TEXT_LIGHT: TextTokens = TextTokens {
    primary: "#0f172a",
    secondary: "#475569",
    muted: "#94a3b8",
};

// ─── Layer 3: Button Component Tokens ────────────────────────────────────────

pub struct ButtonTokens {
    pub primary_bg: &'static str,
    pub secondary_bg: &'static str,
    pub ghost_bg: &'static str,
    pub danger_bg: &'static str,
    pub radius: &'static str,
    pub min_height: &'static str,
}

pub const BUTTON: ButtonTokens = ButtonTokens {
    primary_bg: "var(--gradient-primary)",
    secondary_bg: "var(--surface-elevated)",
    ghost_bg: "transparent",
    danger_bg: "var(--color-danger)",
    radius: "999px",
    min_height: "3rem",
};

// ─── Layer 3: Card Component Tokens ──────────────────────────────────────────

pub struct CardTokens {
    pub radius: &'static str,
    pub border: &'static str,
    pub shadow: &'static str,
    pub padding: &'static str,
}

pub const CARD: CardTokens = CardTokens {
    radius: "var(--radius-2xl)",
    border: "var(--border-subtle)",
    shadow: "var(--shadow-soft)",
    padding: "clamp(1.1rem, 2.6vw, 1.8rem)",
};

// ─── Layer 3: Modal Component Tokens ─────────────────────────────────────────

pub struct ModalTokens {
    pub overlay_bg: &'static str,
    pub panel_radius: &'static str,
    pub panel_width: &'static str,
    pub shadow: &'static str,
    pub z_index: u32,
}

pub const MODAL: ModalTokens = ModalTokens {
    overlay_bg: "rgba(2,6,23,0.55)",
    panel_radius: "var(--radius-2xl)",
    panel_width: "min(100%, 36rem)",
    shadow: "var(--shadow-strong)",
    z_index: 100,
};

// ─── Helpers: Theme Palette Selection ────────────────────────────────────────

pub struct AnimationTokens {
    pub fast:    &'static str,   // 120ms ease
    pub smooth:  &'static str,   // 280ms cubic
    pub bounce:  &'static str,   // 400ms spring
    pub reveal:  &'static str,   // 560ms ease-out
    pub spring:  &'static str,   // 600ms cubic spring
}

pub const ANIMATION: AnimationTokens = AnimationTokens {
    fast: "120ms cubic-bezier(0.4, 0, 0.2, 1)",
    smooth: "280ms cubic-bezier(0.4, 0, 0.2, 1)",
    bounce: "400ms cubic-bezier(0.34, 1.56, 0.64, 1)",
    reveal: "560ms cubic-bezier(0.16, 1, 0.3, 1)",
    spring: "600ms cubic-bezier(0.22, 1, 0.36, 1)",
};

/// تحديد مقياس اللون بناءً على اسم اللوح
pub fn palette_by_name(name: &str) -> &'static ColorScale {
    match name {
        "indigo"  => &INDIGO,
        "emerald" | "green" => &EMERALD,
        "zinc" | "slate" | "gray" => &ZINC,
        "rose"    => &ROSE,
        "cyan"    => &CYAN,
        "violet"  => &VIOLET,
        _ => &INDIGO,
    }
}

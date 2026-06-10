# Amana Examples Gallery

This folder contains 5 premium standalone examples. Each one is designed to demonstrate different layout engines, high-end design aesthetics, micro-animations, and database connectivity.

## Files

| File | Title & Aesthetic | Layout Primitives | Models & Actions |
| --- | --- | --- | --- |
| `01_saas_aura.amana` | SaaS Analytics Control Panel (Dark Neon Glass) | `split` (Hero), `bento` (Operations Board), `asymmetric` | `Metric`, `Feedback` |
| `02_maison_luxe.amana` | MaisonLuxe Luxury Atelier (Light Serif Editorial) | `split-diagonal` (Hero), `magazine` (Catalogue), `editorial` | `CollectionItem`, `Booking` |
| `03_vortex_console.amana` | Vortex Console Room (Obsidian Developer Tech) | `command-center` (Hero Terminal), `masonry` (Diagnostics) | `DeployLog`, `AccessRequest` |
| `04_nova_creative.amana` | Nova Creative Studio (Deep Orchid Motion Blobs) | `asymmetric` (Hero), `showcase-rail` (Portfolio), `split` | `ProjectBrief` |
| `05_cura_wellness.amana` | Cura Wellness Platform (Teal/Mint Healing Day) | `split` (Hero), `sidebar` (Specialties), `bento` (Directory) | `Appointment` |

## Verify One

```powershell
cargo run -- check examples\01_saas_aura.amana --json
```

## Verify All

You can build all examples and run safety/syntax verification with:

```powershell
powershell -File C:\Users\Lenovo\.gemini\antigravity\brain\485c309b-8c61-4267-80cf-8c1243d14b60\scratch\verify-all-examples.ps1
```


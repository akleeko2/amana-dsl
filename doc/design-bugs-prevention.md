# Amana Design Bugs & Layout Prevention Guide

This document lists visual and layout issues encountered in the Amana DSL and generated templates, explaining how they were fixed. Any AI creating or modifying templates in Amana must adhere to these rules to ensure perfect design rendering across all screen sizes.

---

## 1. Character Splitting in Cursive Scripts (e.g. Arabic)
- **Problem**: In narrow viewports, connected Arabic letters were split vertically (e.g. `م س ت خ د م ي` instead of `مستخدمي`). This was caused by the compiler's global CSS rule using `overflow-wrap: anywhere;` on all inline/text elements, which aggressively breaks words at any character boundary.
- **Solution**: Changed the default compiler rule in `src/codegen/express.rs` to use `overflow-wrap: break-word;` (or specifically target text paragraphs with `word-break: normal; overflow-wrap: break-word;`).
- **Rule for AI**: Never use `overflow-wrap: anywhere;` or `word-break: break-all;` on elements displaying RTL or cursive text. Always default to `overflow-wrap: break-word;` and `word-break: normal;`.

---

## 2. Scrollbar Clashes & Sidebar Clipping (Double Scrolling)
- **Problem**: When a page had a header and two sidebars, setting `position: sticky; height: calc(100vh - 56px);` on the sidebars while allowing the central feed/page to scroll resulted in sidebars behaving erratically, double scrollbars appearing, or sidebars being cropped off at the bottom, making it impossible to scroll to the end of sidebar contents.
- **Solution**: 
  - Restrained the outer wrapper (`.fb-wrapper`) to `height: 100vh; overflow: hidden;` to disable browser window scroll.
  - Constrained the main area wrapper (`.fb-main-container`) to `height: calc(100vh - 56px); overflow: hidden;`.
  - Configured each individual column (`.left-sidebar`, `.right-sidebar`, and `.feed-area`) to have `height: 100%; overflow-y: auto;`.
- **Rule for AI**: For multi-column desktop application layouts (like dashboards or social feeds), avoid body-level scrolling. Always enforce `height: 100vh; overflow: hidden` on the main page wrapper, and delegate scrolling independently to each sub-column (`height: 100%; overflow-y: auto`).

---

## 3. Selector Whitespace Collapse (Compiler Parser Bug)
- **Problem**: The Amana stylesheet parser collapsed combinators and spaces inside selectors, parsing `.feed-area > article.amana-card` to `.feed-areaarticle.amana-card` and `.feed-area .amana-card` to `.feed-area.amana-card`, breaking styling overrides.
- **Solution**: Avoided using child combinators (`>`) or class-to-class descendants (`.class1 .class2`). Instead, used descendant selectors containing tag names (e.g., `.feed-area article`), which parse correctly because both parts are identifier-like tokens.
- **Rule for AI**: To override layout element styles compiled from custom components, target using tag names (e.g., `.parent-class tag-name`) to prevent the parser from minifying the whitespace between selectors.

---

## 4. `!important` Rule Conversion
- **Problem**: The Amana parser translates the exclamation operator `!` to the word `not` in CSS style blocks, converting `!important` to `not important` which is invalid CSS and ignored by browsers.
- **Solution**: Bypassed using `!important` by writing selectors with higher specificity, or by utilizing inline style attributes on tags.
- **Rule for AI**: Never write `!important` in `.amana` style blocks. Raise selector specificity (e.g., prefixing parent classes/tags) or inject inline `style="..."` attributes on the HTML tags to guarantee overrides.

---

## 5. CSS `calc()` Space Strip
- **Problem**: Standard spaces inside CSS expressions like `calc(100vh - 56px)` are stripped by the compiler to `calc(100vh-56px)`, which triggers browser errors because subtraction/addition operators inside `calc()` must be padded with whitespace.
- **Solution**: Passed the style as a literal tag attribute string: `(style: "height: calc(100vh - 56px);")` which bypasses CSS parsing.
- **Rule for AI**: To write standard math formulas like `calc(...)` in stylesheets safely without operator spaces being stripped, pass them as literal strings in HTML/View attributes, not in the `style:` DSL block.

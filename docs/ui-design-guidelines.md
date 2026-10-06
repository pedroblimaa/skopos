# UI design guidelines

Read before implementing or reviewing UI, styling, shared controls, tokens, tooltips, motion, or accessibility changes.

Skopos uses an Apple-inspired desktop design, adapted to its React/Tauri shell. Prioritize clarity, visual hierarchy, and familiar interaction. These are project choices informed by Apple's guidance, not a requirement to reproduce native Apple controls.

## Design decisions

- Before a substantial UI change, write a brief plan: the user's task, primary action, information hierarchy, layout, and existing tokens/components to reuse. Check it against the requested flow before coding. Small fixes need only a focused decision.
- Ground screens in Skopos's actual content: product names, BRL price ceilings, Telegram sources, promotion messages, and session states. Make product names and price ceilings easy to scan; keep source and message context easy to inspect.
- Keep the established palette and typography. A distinctive interface comes from clear product information and deliberate hierarchy; introducing fonts, colors, or decoration requires a concrete reason within the requested scope.

## Visual system

- Keep the interface calm: ink violet backgrounds, opaque content surfaces, warm porcelain text, and a seafoam accent for primary actions, links, and focus. Reserve red for errors and destructive actions.
- Define all literal colors in `src/color-scheme.css`; keep radii, shadow geometry, fonts, spacing, and motion tokens in `src/global.css`. Reuse them across pages and components rather than introducing local palette variations.
- Always use CSS color variables outside `src/color-scheme.css`, including shadow colors, SVG/QR colors, transparency, and scrollbars. `pnpm lint:colors` enforces this for frontend source and runs as part of `pnpm lint`, `pnpm check`, and `pnpm build`. Keep the palette distinct from Apple; use Apple only as a reference for hierarchy and interaction.
- Use the bundled Geist font through `--font-system`, with the existing platform fallbacks. Build hierarchy with size, weight, and spacing: page titles around 28-32px, section headings 16-18px, controls/body 13-15px, secondary labels at least 12px. Avoid uppercase tracking for ordinary form labels.
- Align related content and use a consistent 4px spacing rhythm. Give sections more space than controls within a section. Keep forms constrained to a readable width and let narrow windows wrap naturally.
- Use rounded corners consistently: controls around 14px, content panels around 22px. Prefer subtle separators and surface differences to heavy outlines. Keep content shadows modest; reserve stronger depth for menus and popovers.
- Use restrained translucency and blur for navigation and floating menus only. Keep forms and product content opaque and legible. Avoid stacking glass surfaces or adding decorative gradients behind ordinary content.
- Use cards for meaningful groups, separators for relationships, and numbering for actual sequences. Avoid repeated decorative labels or identical cards around unrelated content.
- Recessed results must be darker than the product card above them while remaining distinct from the page background. Keep hover fills within their rounded borders. Use `--page-content-width` for main lists; constrain forms separately.

## Controls and interface copy

- Keep one prominent primary action per task. Use quiet secondary actions, concise sentence-case labels, and the user-facing vocabulary Products, Add product, Edit product, Product name, and Alternative name.
- Name actions precisely and consistently through buttons, confirmations, and feedback. Describe failures and recovery steps plainly; empty states should identify the next useful action.
- Use shared `InfoTooltip` variants: `information` appears immediately for passive status/help; `action` delays hover hints for clickable controls. Both appear immediately on keyboard focus, support Escape dismissal, and remain readable while hovered. Passive icons keep a subdued tone; use the empty-result tone for no matches. Keep required instructions and validation visible.
- Product cards open editing across the card while preserving independent controls. Use shared `Dialog` for destructive confirmations, with focus management and cancellation blocked during the write.
- Show price, source, date/time, copy link, and a short description in promotion previews. Reveal full text and optional photos on expansion. Zero matches use an information icon beside the product name rather than an empty results drawer.
- Search for reusable components and styles before adding UI. Feature CSS may arrange shared controls but should not redefine their visual contract. Extend a shared component with a deliberate variant when needed.
- Extend shared controls for new visual states instead of restyling them from page CSS. Review login, product list, form, toolbar, and floating panels together when changing shared tokens.

## Motion and accessibility

- Animate feedback rather than decoration: hover/focus changes around 160-180ms, page entrances around 220ms, small travel distances, and no bounce. Respect `prefers-reduced-motion`; never delay an action to finish an animation.
- Keep page entrances restrained; avoid separate entrance animations for every row or section.
- Preserve visible keyboard focus, text contrast, clear disabled/error states, and accessible names for icon controls. Use existing Lucide icons with consistent size and optical alignment.

## Visual review

- Inspect the rendered interface, using screenshots when available, at the normal desktop size and a narrow window. Review realistic long product/account names and the loading, empty, error, and busy states affected by the change.
- Check whether hierarchy makes the next action clear, then remove unnecessary decoration. Verify keyboard focus, tooltip/menu behavior, and reduced motion. Automated test success does not replace visual inspection.
- Review only the changed flow and shared surfaces it affects. Do not use visual critique as authorization for an unrelated redesign.

Sources: [Anthropic frontend-design skill](https://github.com/anthropics/skills/blob/main/skills/frontend-design/SKILL.md), [Apple layout guidance](https://developer.apple.com/design/human-interface-guidelines/layout), [Apple color guidance](https://developer.apple.com/design/human-interface-guidelines/color), [Meet Liquid Glass](https://developer.apple.com/videos/play/wwdc2025/219/), and [Apple accessibility guidance](https://developer.apple.com/design/human-interface-guidelines/accessibility). The planning, structural restraint, copy, and critique guidance adapts Anthropic's principles to Skopos's existing desktop design constraints; the bundled font, palette, controls, and motion tokens remain project requirements.

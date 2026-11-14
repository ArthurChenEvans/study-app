# Shudhana Design Rules

The rules every screen follows. Since 12 Nov 2026 the look is the playful one from `design/playful-redesign/` (chunky, game-like). Since 13 Nov 2026 the colours are Rust's, from the Rust Foundation brand guide, since the app is built in Rust. Built from Refactoring UI, Nielsen's 10 usability heuristics, Laws of UX, WCAG 2.2 and Butterick's Practical Typography. When a rule and a "looks nice" instinct disagree, the rule wins until the rule is changed here.

## 1. Core principles

1. **Hierarchy before decoration.** Every screen has one primary thing. Make it obvious with size, weight and contrast, then de-emphasise everything else (Refactoring UI: emphasise by de-emphasising).
2. **Design the feature, not the layout.** Start from the job (write a question, answer a question), not from a page shell with a navbar and a hero.
3. **Constrained choices only.** Spacing, type sizes, colours, radii and shadows come from the scales below. No one-off values.
4. **Colour is a shared resource.** Most of the UI is warm off-white (light) or Rust dark blue (dark) neutrals. Colour is spent on actions and states (the main action in Rust orange, right, wrong, guessed, due in amber), never on decoration.
7. **Chunky, not loud.** Thick 2px borders, big radii and a pressed-down ledge under buttons make things feel like they can be picked up. That's the whole playfulness budget: no confetti, no bounce, no mascots.
5. **Group with space, not boxes.** Proximity and whitespace separate content first. Borders and cards are a last resort.
6. **Keyboard first in the quiz.** The quiz is the most used screen and should be fully usable without the mouse.

## 2. AI-generated UI tells, and what to do instead

| Tell | Do instead |
|---|---|
| Purple/indigo gradients, neon accents | Palette from section 5. Flat colour, no gradients |
| Everything in a card, cards inside cards | Whitespace and headings to group. Cards only for things you act on as a unit (a topic in a list) |
| Emojis as icons or bullets | One icon set (Lucide), used only where an icon adds meaning, always with a label or tooltip |
| Glows, glassmorphism, heavy drop shadows | Depth from surface levels and 2px borders. The only "shadow" on a button or tile is the solid ledge under it. One soft shadow for floating things (menus, dialogs) |
| Centred hero, big tagline, three feature cards | It's a tool, open straight onto the work (topic list or last quiz) |
| Every element the same weight and size | Clear type scale, at most 3 sizes on one screen plus headings |
| Grey text on coloured backgrounds | Text on colour uses that colour's `-content` token |
| Coloured side stripes on every block | Colour only where it marks a state |
| Status dots and badges that mean nothing | Every badge maps to a real state and has a text label |
| Default Inter/system font everywhere, 16px for everything | The type pairing and scale in section 4 |
| Rounded-full pill buttons for everything | Radii from the tokens (14px buttons, 12px fields, 16px tiles), pills only for counts |
| Placeholder text as the only label | Visible labels above every field |
| "Lorem ipsum" density, giant padding | Padding from the spacing scale, sized to the content |

## 3. Spacing scale

Base unit 4px. Only these values: **4, 8, 12, 16, 24, 32, 48, 64, 96**.

Tailwind classes: `1, 2, 3, 4, 6, 8, 12, 16, 24`.

- Inside a component (icon to label, label to field): 4 to 8
- Between related items (fields in a form, options in a question): 12 to 16
- Between groups (form sections, question to options): 24 to 32
- Between page regions: 48+
- Space around a group is always bigger than space inside it, so groups read as groups.

## 4. Typography

- **UI and body:** Nunito (rounded, friendly, very readable at heavy weights). **Code, key hints, file paths:** IBM Plex Mono. **Maths:** KaTeX's own fonts. Both fonts ship with the app (`@fontsource`), nothing loads from the internet.
- **Scale (px):** 12, 13, 14, 15, 16, 18, 20, 24, 28, 30, 36. 13 is for uppercase labels, 15 for body inside tiles and rows.
  - 12 to 13: uppercase labels (buttons, nav, tabs, chips, table headers) and metadata
  - 14 to 15: secondary text, table cells, body inside tiles
  - 16: body, form inputs
  - 18: quiz options and card answers
  - 24 to 30: question text in the quiz, page titles
  - 36+: the score on Results, the one headline number on Overview
- **Weights:** 500 for body, 700 for emphasis and rows, 800 for labels and headings, 900 for page titles and big numbers.
- **Uppercase labels:** buttons, sidebar links, tabs, chips and table headers are uppercase with 0.05 to 0.08em tracking, done in CSS (`text-transform`), so the words in the page and for screen readers stay in sentence case.
- **Line height:** 1.5 for body, 1.2 for headings and big numbers.
- **Line length:** 45 to 90 characters (Practical Typography). Question and explanation text gets `max-w-prose`.
- **Hierarchy with colour too:** primary text `base-content`, secondary text the muted shade (section 5). Not lighter font sizes alone.
- Numbers in tables and timers use tabular figures (`tabular-nums`) so they don't jump.

## 5. Colour palette

Rust's brand colours for the general theme (Rust Dark Blue #1E2650, Rust Orange #D34516, Rust Blue #28607F, Rust Silver #67737A), one main-action colour, amber for anything due, and a right / wrong / guessed set with a colour-blind safe alternative. Every text pairing below was checked against WCAG 2.2 AA (4.5:1 for text, 3:1 for UI component boundaries).

### General theme (Rust)

| Token | Light | Dark | Use | Contrast (light / dark) |
|---|---|---|---|---|
| base-100 | #f6f3ef warm off-white | #1e2650 Rust dark blue | Page | |
| base-200 | #eeeae5 | #171d40 | Sidebar, panes, hover | |
| base-300 | #ded8d0 | #3d4775 | Borders, dividers, empty bar track | decorative only |
| surface | #ffffff | #283163 | Tiles, rows, fields, dialogs | |
| base-content | #1e2650 Rust dark blue | #f6f3ef | Main text | 14.5 / 11.1 on surface |
| muted | #5a656c | #c3c8da | Secondary text | 6.0 / 7.4 on surface |
| field border | #67737a Rust silver | #7d86b0 | Input and checkbox borders | 4.9 / 3.5 on surface |
| primary | #d34516 Rust orange, #ffffff on it | same | The one main action, the logo, today's ledge | label 4.5 |
| primary ink | #b03a12 | #f7915f | Orange as text, links, tabs, focus rings | 6.1 / 5.4 on surface |
| accent (due) | oklch(78% 0.13 78), dark text | oklch(80% 0.13 80) | Due counts, due chips, "to review" | |
| neutral | #1e2650, #ffffff on it | #f6f3ef, #1e2650 on it | Selected segment, chosen chip | |
| info | #28607f Rust blue | #8cc4e2 | Neutral notices, links to sources, chart lines | 6.8 / 6.5 on surface |

Rust orange as text is just under 4.5:1 on the light page, and too dark on the dark page, so orange text, links and focus rings use the primary ink (darker in light, lighter in dark). Chart lines use Rust blue rather than orange, so a score trend never reads as wrong.

### Right, wrong and guessed

A setting in the sidebar switches between the standard pair and the colour-blind safe pair. The default is standard.

| | Standard light | Standard dark | Colour-blind light | Colour-blind dark |
|---|---|---|---|---|
| Right fill / text on it | #0f7b55 / #fff | #35b37e / #06140d | #025884 / #fff | #0573aa / #fff |
| Right ink | #0b6645 (6.9) | #7fe0b3 (7.4) | #004d77 (8.9) | #87c6f2 (6.4) |
| Wrong fill / text on it | #b81f3f / #fff | #f0587a / #1a0509 | #cc6c13 / #140d08 | #d67523 / #140d08 |
| Wrong ink | #9e1735 (8.0) | #ff9fb2 (6.4) | #853900 (8.1) | #f9b379 (6.6) |
| Guessed tint / ink | #c5e8d6 / #0a5238 (7.0) | #173d2e / #b6f0d4 (9.4) | #c1dcf0 / #013658 (8.8) | #163c54 / #bbddf6 (8.2) |

Ink contrasts are on the surface. Fills carry their own text colour, at least 5.2:1.

Rules:
- Right, wrong and guessed are **never shown by colour alone**. Always an icon and a word ("Correct", "Wrong", "Guessed"), and guessed is always hatched. That's what makes the standard green and red acceptable: the greyscale test and the colour-blind switch both still work.
- In the standard pair a wrong answer and a destructive action share red. Destructive actions only appear inside a confirmation dialog that names what goes, so the two never sit side by side.
- Primary (Rust orange) is never used for right or wrong, so "this is the main button" and "you got it right" can't be confused. The standard wrong is a crimson rather than a plain red, so it stays clearly apart from the orange. The colour-blind wrong is still orange (the blue and orange pair is what makes it safe), so there the icon and word carry the difference.
- Amber means due and nothing else. It stays off decoration.
- No gradients (the guessed hatch is a pattern, not a gradient). No pure black.
- Dark is the default, with the same token names. See section 11.

## 6. Component rules

**Buttons**
- Uppercase, 800 weight, 48px tall (38px small), 14px radius (12px small), with a solid 4px ledge underneath in the button's own colour mixed 70% with black. Pressing moves it down onto the ledge.
- One primary (Rust orange, `btn-primary`) per screen or dialog. Everything else is the ghost style: surface fill, a 2px border-coloured ring and a border-coloured ledge (`btn-ghost`, `btn-outline`). Quiet text buttons (`btn-link`, menu items) have no ledge.
- Right and wrong coloured buttons are for answering only (Continue on the feedback sheet, card ratings).
- Destructive actions are `btn-error` and only appear inside a confirmation dialog, never as the first button you see.
- Labels are verbs ("Start quiz", "Save question"), never "OK" or "Submit".

**Tiles, chips and rows**
- A tile: surface fill, 2px base-300 border, 16px radius, no shadow. Something you press as a whole (a quiz option, a rating) also gets a 2px ledge.
- A chip: uppercase 12px, 800 weight, 8px radius, a 16% tint of its colour with that colour's ink for text. Chips always say a word, and only state colours (due, right, wrong, guessed, info) or muted.
- List rows are separated by 2px base-300 rules. Table headers are uppercase 12px muted labels.
- Counts in the sidebar sit in an amber pill when something is due.

**Forms (question editor)**
- Visible label above every field, help text below in muted 14px.
- Errors appear next to the field, in error colour with an icon, saying how to fix it ("Mark at least one correct option").
- Option rows: checkbox for "correct", the text, then a remove button. Add-option button disabled at 5, remove disabled at 3.

**Quiz screen**
- One question at a time, centred column at `max-w-prose`. Nothing else competes with it.
- Options are big tiles with a ledge (full row clickable, at least 60px tall, Fitts's law), numbered 1 to 5 in a key box to match the keyboard shortcuts.
- Feedback is a bottom sheet: verdict, explanation and Continue share one band tinted in the right or wrong colour, with a big round icon and the word.
- Timer top right in tabular figures, switches to warning colour in the last 20%.
- Flag ("I guessed") sits next to the submit button, toggled with F.

**Results screen**
- The score is the biggest thing on screen (30 to 36px), with "N guessed" right below it.
- Per-topic breakdown as a simple table, not charts, until there's history to chart.
- Wrong and guessed questions listed first, each with its explanation and the wrong-answer reason picker.

**Split editor (preview and question form)**
- Resizable split, preview left on base-200, form right on base-100.
- File tabs above the preview, styled as tabs, not cards.

**Lists and search**
- Topics, questions and sources are rows in tables or lists, not card grids.
- Search updates as you type, with category tabs and sortable column headers (SkyUI model).
- Empty states say what to do next and have one button that does it ("No topics yet. Create your first topic").

**Sliders:** a 6px track in base-300 with the filled part in primary ink, a 16px round thumb in primary with a surface ring, and a 24px tall hit area. Don't use DaisyUI's default `range`: its fill is as tall as the thumb, which reads as a heavy slab (near-white in the dark theme). Show the value as text next to the slider.

**Nothing bright and wide unless it's the point.** In the dark theme a full-width or long light shape (a filled bar, a light chip, a floating toolbar) pulls the eye more than anything else on the page. Reserve light fills for the one primary action and small marks. Current-position markers (like the quiz track) are outlined, not filled.

**Icons:** Lucide at a heavier 2.4 stroke, 16px inline with text, 20 to 24px standalone and in nav, always with a label or tooltip. Section icons in the sidebar take that section's colour.

**Motion:** 150ms ease-out for hovers, toggles and the button press, 200ms for dialogs and the feedback sheet sliding up. Nothing bounces. Respect `prefers-reduced-motion`.

## 7. Reference apps

- **Anki (review screen):** one card, nothing else, and every answer button has a keyboard shortcut and shows its consequence. Copy the focus and the shortcuts. Avoid its dated, cramped settings screens.
- **Obsidian:** Markdown, LaTeX and code all rendered in one consistent style, plus resizable split panes and a command palette. Copy the split pane behaviour and the rendering consistency.
- **Linear:** restrained colour (almost all neutral, colour only for status), dense but calm lists, keyboard shortcuts everywhere, and fast as-you-type search. Copy the list density and the colour discipline.
- **Zotero:** the bibliography model. Structured fields, one library cited from many places, and "where is this used" views. Copy the global source library with backlinks to topics.

## 8. Review checklist (run on every screen)

**Look**
- [ ] Squint test: blur your eyes, the one most important thing is still obvious
- [ ] Greyscale test (DevTools > Rendering > emulate achromatopsia): hierarchy and every state still readable
- [ ] Every spacing, size and colour value comes from the scales above
- [ ] No tells from section 2
- [ ] At most one primary button

**Accessibility (WCAG 2.2 AA)**
- [ ] Text contrast at least 4.5:1 (3:1 for 18px+ bold or 24px+)
- [ ] Field borders, focus rings and icons at least 3:1 against their background
- [ ] Everything reachable and usable by keyboard, with a visible focus ring
- [ ] Click targets at least 24x24px (44px for quiz options)
- [ ] State never shown by colour alone

**Nielsen's 10 heuristics**
- [ ] 1. Visibility of system status (saving, timers, file loading, progress through the quiz)
- [ ] 2. Match with the real world (plain words: "topic", "guessed", not internal terms)
- [ ] 3. User control and freedom (undo or cancel, leave a quiz safely)
- [ ] 4. Consistency and standards (same action, same place and style, every screen)
- [ ] 5. Error prevention (disable impossible actions, confirm destructive ones)
- [ ] 6. Recognition rather than recall (show options and recent items, don't make you remember IDs)
- [ ] 7. Flexibility and efficiency (keyboard shortcuts for frequent actions)
- [ ] 8. Aesthetic and minimalist design (nothing on screen that doesn't help the task)
- [ ] 9. Help users recognise and recover from errors (say what went wrong and how to fix it)
- [ ] 10. Help and documentation (short hints where a feature isn't obvious)

**Laws of UX spot checks**
- [ ] Hick's law: quiz setup shows sensible defaults so you aren't facing every choice at once
- [ ] Fitts's law: frequent targets are big and near where your attention already is
- [ ] Jakob's law: common patterns (tabs, dialogs, search) behave how they do in other apps

## 9. Data visualisation

Charts show progress from saved attempts only. They never invent data the app doesn't store.

**Colours**
| Token | Value | Job |
|---|---|---|
| `--chart-right` | the right fill (section 5) | Right and sure |
| `--guessed-fill` | 135° hatch, right fill 2px on the guessed tint 4px | Right but flagged as guessed |
| `--chart-wrong` | the wrong fill | Wrong |
| `--chart-ink` | info, Rust blue (#28607f light, #8cc4e2 dark) | Any single-series chart (score trend, reason counts, forecast) |
| `--chart-grid` | base-300 | Gridlines, hairline and solid |

Both palettes keep right darker than wrong in greyscale, and guessed adds the hatch, so the three separate without colour. Labels inside fills use the fill's own text colour, and guessed counts sit on a small surface-coloured label because text over the hatch is hard to read.

**Forms**
- Score over time: one line, 2px, 8 to 10% area wash, filled dots with a 2px surface ring, hollow dots for retries. Only the latest point is labelled.
- Part of a whole (right / guessed / wrong): thin segmented bar (6 to 10px) in lists, tall segmented bar (28px, counts inside when they fit) as the hero of Results.
- Ranked counts (wrong-answer reasons): horizontal bars, 12px thick, square at the baseline, 4px rounded end, value at the tip.
- One headline number per view (accuracy on Overview, score on Results), 48px or larger, proportional figures.

**Behaviour**
- Every mark has a hover tooltip. The line chart has a crosshair and works with the arrow keys.
- Every chart with more than one series has a legend. Every chart has a "Show as table" view.
- Text in and around charts uses text colours, never the series colour.

## 10. Surfaces

- Page: base-100. Sidebar: base-200, with a 2px base-300 border.
- Tile (`.panel`): surface fill, 2px base-300 border, 16px radius, no shadow. Used for things read or pressed as a unit: charts, stat tiles, the quiz summary, grouped settings, tables.
- Nested radii: things inside a tile use 12px or less.
- The one hero on a screen (Overview's due banner) is an orange tile with a ledge.

## 11. Dark theme

Dark is the default. A Dark / Light switch sits at the bottom of the sidebar and the choice is remembered on the device. Dark is Rust's dark blue, not an inversion: raised things get lighter (sidebar #171d40, page #1e2650, tiles #283163). Values and contrasts are in section 5.

Code blocks use Shiki's min-dark theme in dark and min-light in light. Some of their token colours are under 4.5:1 (light comments are 1.7:1), so `src/lib/markdown.ts` mixes any failing colour toward black (light) or white (dark), keeping its hue, until it clears 4.6:1 on base-100 in light and the panel in dark. Both themes ship as CSS variables, so the switch recolours code without rendering it again.

## Sources

- Wathan, A. and Schoger, S. *Refactoring UI*. https://www.refactoringui.com
- Nielsen, J. 10 Usability Heuristics for User Interface Design. https://www.nngroup.com/articles/ten-usability-heuristics/
- Yablonski, J. Laws of UX. https://lawsofux.com
- W3C. Web Content Accessibility Guidelines (WCAG) 2.2. https://www.w3.org/TR/WCAG22/
- Butterick, M. *Practical Typography*. https://practicaltypography.com
- The Fountain Institute. 7 Signs a UI Has Been Vibe Coded. https://www.thefountaininstitute.com/blog/signs-vibe-coded-ui
- daisyUI theme reference. https://daisyui.com/SKILL.md
- Rust Foundation brand guide (colours). https://rustfoundation.org/brand-guide
- Nunito. https://fonts.google.com/specimen/Nunito

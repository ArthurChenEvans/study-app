# Study App Design Rules

The rules every screen follows. Built from Refactoring UI, Nielsen's 10 usability heuristics, Laws of UX, WCAG 2.2 and Butterick's Practical Typography. When a rule and a "looks nice" instinct disagree, the rule wins until the rule is changed here.

## 1. Core principles

1. **Hierarchy before decoration.** Every screen has one primary thing. Make it obvious with size, weight and contrast, then de-emphasise everything else (Refactoring UI: emphasise by de-emphasising).
2. **Design the feature, not the layout.** Start from the job (write a question, answer a question), not from a page shell with a navbar and a hero.
3. **Constrained choices only.** Spacing, type sizes, colours, radii and shadows come from the scales below. No one-off values.
4. **Colour is a shared resource.** Most of the UI is warm neutrals. Colour is spent on actions and states (primary action, correct, wrong, guessed), never on decoration.
5. **Group with space, not boxes.** Proximity and whitespace separate content first. Borders and cards are a last resort.
6. **Keyboard first in the quiz.** The quiz is the most used screen and should be fully usable without the mouse.

## 2. AI-generated UI tells, and what to do instead

| Tell | Do instead |
|---|---|
| Purple/indigo gradients, neon accents | Palette from section 5. Flat colour, no gradients |
| Everything in a card, cards inside cards | Whitespace and headings to group. Cards only for things you act on as a unit (a topic in a list) |
| Emojis as icons or bullets | One icon set (Lucide), used only where an icon adds meaning, always with a label or tooltip |
| Glows, glassmorphism, heavy drop shadows | Depth from surface levels (base-100, 200, 300). One shadow level for floating things (menus, dialogs) |
| Centred hero, big tagline, three feature cards | It's a tool, open straight onto the work (topic list or last quiz) |
| Every element the same weight and size | Clear type scale, at most 3 sizes on one screen plus headings |
| Grey text on coloured backgrounds | Text on colour uses that colour's `-content` token |
| Coloured side stripes on every block | Colour only where it marks a state |
| Status dots and badges that mean nothing | Every badge maps to a real state and has a text label |
| Default Inter/system font everywhere, 16px for everything | The type pairing and scale in section 4 |
| Rounded-full pill buttons for everything | Radii from the tokens, pills only for tags |
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

- **UI and body:** IBM Plex Sans. **Code:** IBM Plex Mono. **Maths:** KaTeX's own fonts. Same family for sans and mono keeps them matched.
- **Scale (px):** 12, 14, 16, 18, 20, 24, 30, 36. Nothing else.
  - 12: metadata only (timestamps, counts)
  - 14: secondary text, table cells, labels
  - 16: body, form inputs, options
  - 18 to 20: question text in the quiz
  - 24 to 36: page titles, the score on the results screen
- **Weights:** 400 and 600 only. Use 600 sparingly.
- **Line height:** 1.5 for body, 1.25 for headings and the big numbers.
- **Line length:** 45 to 90 characters (Practical Typography). Question and explanation text gets `max-w-prose`.
- **Hierarchy with colour too:** primary text `base-content`, secondary text the muted shade (section 5). Not lighter font sizes alone.
- Numbers in tables and timers use tabular figures (`tabular-nums`) so they don't jump.

## 5. Colour palette

Warm paper-and-coffee neutrals with one primary and four state colours. Every text pairing below was checked against WCAG 2.2 AA (4.5:1 for text, 3:1 for UI component boundaries).

| Token | Value | Hex | Use | Contrast |
|---|---|---|---|---|
| base-100 | oklch(97% 0.010 80) | #f9f4ee | Main background | |
| base-200 | oklch(94% 0.014 78) | #f0eae1 | Sidebars, the preview pane, hover | |
| base-300 | oklch(89% 0.018 75) | #e2d9ce | Dividers, selected rows | decorative only |
| base-content | oklch(27% 0.025 60) | #30241a | Main text | 13.9 on base-100 |
| muted (custom) | oklch(42% 0.022 65) | #564b41 | Secondary text | 7.8 on base-100, 7.1 on base-200 (AAA) |
| field border (custom) | oklch(58% 0.02 65) | #83786e | Input and checkbox borders | 3.9 on base-100 |
| primary | oklch(40% 0.065 50) | #643d26 | The one main action per screen, focus rings | 8.6 with its content |
| neutral | oklch(32% 0.02 60) | #3b3129 | Secondary solid buttons, tooltips | 10.7 with its content |
| accent / warning | oklch(78% 0.13 78) | #e5ac4c | "Guessed" flag, timer running low | 7.5 with dark content |
| success | oklch(42% 0.11 145) | #1c5c23 | System success, score going up | 7.4 (AAA) |
| error | oklch(44% 0.15 30) | #942318 | Validation errors, destructive actions | 7.7 (AAA) |
| info | oklch(44% 0.07 235) | #265874 | Neutral notices, source/citation links | 7.0 (AAA) |
| right (custom) | #025884, text #004d77 | | Quiz answer right | text 8.3 (AAA) |
| wrong (custom) | #cc6c13, text #853900 | | Quiz answer wrong | text 7.5 (AAA) |
| guessed (custom) | hatch of #025884 on #c1dcf0, text #013658 | | Right but guessed | text 8.9 on its tint |

Rules:
- Quiz outcomes use the blue / orange pair, not green / red, so they survive colour blindness. Green and red stay for system messages only.
- Correct, wrong and guessed are **never shown by colour alone**. Always add an icon and a word ("Correct", "Wrong", "Guessed") for colour-blind use and the greyscale test.
- Primary is not used for correct/wrong, so "this is the main button" and "you got it right" never get confused.
- No gradients. No pure black or pure white.
- Dark theme comes later, as a second theme block with the same token names.

### DaisyUI theme

```css
@plugin "daisyui/theme" {
  name: "studyapp";
  default: true;
  prefersdark: false;
  color-scheme: light;

  --color-base-100: oklch(97% 0.010 80);
  --color-base-200: oklch(94% 0.014 78);
  --color-base-300: oklch(89% 0.018 75);
  --color-base-content: oklch(27% 0.025 60);
  --color-primary: oklch(40% 0.065 50);
  --color-primary-content: oklch(97% 0.010 80);
  --color-secondary: oklch(32% 0.02 60);
  --color-secondary-content: oklch(94% 0.014 78);
  --color-accent: oklch(78% 0.13 78);
  --color-accent-content: oklch(27% 0.025 60);
  --color-neutral: oklch(32% 0.02 60);
  --color-neutral-content: oklch(94% 0.014 78);
  --color-info: oklch(44% 0.07 235);
  --color-info-content: oklch(97% 0.010 80);
  --color-success: oklch(42% 0.11 145);
  --color-success-content: oklch(97% 0.010 80);
  --color-warning: oklch(78% 0.13 78);
  --color-warning-content: oklch(27% 0.025 60);
  --color-error: oklch(44% 0.15 30);
  --color-error-content: oklch(97% 0.010 80);

  --radius-selector: 0.25rem;
  --radius-field: 0.375rem;
  --radius-box: 0.5rem;

  --size-selector: 0.25rem;
  --size-field: 0.25rem;

  --border: 1px;

  --depth: 0;
  --noise: 0;
}

@theme {
  --color-muted: oklch(42% 0.022 65);
  --color-field-border: oklch(58% 0.02 65);
  --font-sans: "IBM Plex Sans", ui-sans-serif, sans-serif;
  --font-mono: "IBM Plex Mono", ui-monospace, monospace;
}
```

`--depth: 0` turns off DaisyUI's built-in bevel shadows, which are one of the things that make DaisyUI apps look the same as each other.

## 6. Component rules

**Buttons**
- One primary (`btn-primary`) per screen or dialog. Everything else is `btn-ghost` or `btn-outline`.
- Destructive actions are `btn-error` and only appear inside a confirmation dialog, never as the first button you see.
- Labels are verbs ("Start quiz", "Save question"), never "OK" or "Submit".

**Forms (question editor)**
- Visible label above every field, help text below in muted 14px.
- Errors appear next to the field, in error colour with an icon, saying how to fix it ("Mark at least one correct option").
- Option rows: checkbox for "correct", the text, then a remove button. Add-option button disabled at 5, remove disabled at 3.

**Quiz screen**
- One question at a time, centred column at `max-w-prose`. Nothing else competes with it.
- Options are large click targets (full row clickable, at least 44px tall, Fitts's law), numbered 1 to 5 to match the keyboard shortcuts.
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

**Icons:** Lucide, 16px inline with text, 20px standalone, always with a label or tooltip.

**Motion:** 150ms ease-out for hovers and toggles, 200ms for dialogs. Nothing bounces. Respect `prefers-reduced-motion`.

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
| `--chart-right` | #025884 | Right and sure |
| `--guessed-fill` | 135° hatch, #025884 2px on #c1dcf0 4px | Right but flagged as guessed |
| `--chart-wrong` | #cc6c13 | Wrong |
| `--chart-ink` | #643d26 | Any single-series chart (score trend, reason counts) |
| `--chart-grid` | oklch(91% 0.016 76) | Gridlines, hairline and solid |

Validator (all pairs): lightness band, chroma floor, CVD separation 22.1, normal-vision 31.5 and 3:1 on the surface all pass. Greyscale lightness: right 0.33, wrong 0.52, guessed tint 0.84 plus hatch, so the three separate without colour. Labels inside fills: white on right (7.7:1), near-black on wrong, and guessed counts sit on a small surface-coloured label because text over the hatch is hard to read.

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

- Page: base-100. Sidebar: base-200.
- Panel (`.panel`): one raised surface, oklch(99% 0.006 80), 1px base-300 border, 12px radius, two-layer soft shadow. Used for things read as a unit: charts, stat tiles, the quiz summary, grouped settings, tables.
- Nested radii: things inside a panel use 8px or less.

## Sources

- Wathan, A. and Schoger, S. *Refactoring UI*. https://www.refactoringui.com
- Nielsen, J. 10 Usability Heuristics for User Interface Design. https://www.nngroup.com/articles/ten-usability-heuristics/
- Yablonski, J. Laws of UX. https://lawsofux.com
- W3C. Web Content Accessibility Guidelines (WCAG) 2.2. https://www.w3.org/TR/WCAG22/
- Butterick, M. *Practical Typography*. https://practicaltypography.com
- The Fountain Institute. 7 Signs a UI Has Been Vibe Coded. https://www.thefountaininstitute.com/blog/signs-vibe-coded-ui
- daisyUI theme reference. https://daisyui.com/SKILL.md

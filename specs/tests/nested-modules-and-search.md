# Test specification: nested modules, related topics, graph and search query language

Traces to `specs/features/nested-modules.feature` and `specs/features/search-query.feature` (R217 to R220). Every test ID below appears in the name or doc comment of the test that covers it.

## Where each test lives

| Layer | Files |
|---|---|
| core | `crates/study-core/tests/tree.rs`, `query.rs`, `query_properties.rs` |
| command and store | `crates/study-api/src/tree_tests.rs`, the nested sections of `sync_tests.rs` and `anki_tests.rs`, `crates/study-store/src/tree_tests.rs` (upgrade) |
| component | `src/components/tree/*.test.ts` |
| e2e | `e2e/nested_modules_test.py` |

T-IMP-01, T-IMP-02 and T-IMP-04 test the deck-to-tree mapping in core (`anki_placement`), and T-IMP-03 tests the import itself in study-api.

Layers: **core** is `crates/study-core` unit and property tests, **store** is `crates/study-store` integration tests, **component** is a Svelte component test, **e2e** is Playwright.

## 1. Module tree rules (core)

| ID | Test | Given | Expect |
|---|---|---|---|
| T-TREE-01 | Submodule is created under parent | root "CA" | child "N2T" has parent "CA" |
| T-TREE-02 | Same name, different parents, both allowed | "N2T", "CS:APP" | two "CPU" topics with different ids |
| T-TREE-03 | Same-named topics share nothing | add a question to N2T › CPU | CS:APP › CPU question count unchanged (store) |
| T-TREE-04 | Sources stay per topic | cite source A on N2T › CPU | CS:APP › CPU has no source A (store) |
| T-TREE-05 | Duplicate name under the same parent is refused | N2T has "CPU" | `DuplicateName`, nothing written |
| T-TREE-06 | Duplicate check ignores case and outer spaces | N2T has "CPU" | " cpu " refused |
| T-TREE-07 | Inner spaces still count | N2T has "CPU Logic" | "CPULogic" allowed |
| T-TREE-08 | Topic can't hold a module | topic id as parent | `ParentIsTopic` |
| T-TREE-09 | Depth 4 allowed | chain of 3 | 4th level created |
| T-TREE-10 | Depth 5 refused | chain of 4 | `TooDeep { limit: 4 }` |
| T-TREE-11 | Empty or whitespace-only name refused | "   " | `EmptyName` |

## 2. Default module (core + component)

| ID | Test | Expect |
|---|---|---|
| T-DEF-01 | Topic with no module goes to Default after confirmation | topic's parent is Default |
| T-DEF-02 | Default is created lazily | no Default until the first topic needs it |
| T-DEF-03 | Default is a root | Default's parent is none, other roots are not its children |
| T-DEF-04 | Default can't be renamed | `ReservedModule` |
| T-DEF-05 | Default can't be moved under another module | `ReservedModule` |
| T-DEF-06 | No submodules in Default | `ReservedModule` |
| T-DEF-07 | User module named "default" refused (any case) | `ReservedName` |
| T-DEF-08 | Topics move out of Default normally | parent updated |
| T-DEF-09 | Warning shows both texts and Yes/No (component) | exact copy from the spec |
| T-DEF-10 | "No" keeps form input and focuses the module picker (component) | values kept, focus on picker |
| T-DEF-11 | "Yes" creates in Default (component) | create called with no module |

## 3. Moving and deleting (core, transactions in store)

| ID | Test | Expect |
|---|---|---|
| T-MOVE-01 | Moving a topic carries questions, sources, schedule and history | all still linked (store) |
| T-MOVE-02 | Move into a parent with a same-named topic | `DuplicateName`, nothing moved |
| T-MOVE-03 | Module into itself | `Cycle` |
| T-MOVE-04 | Module into its own descendant | `Cycle` |
| T-MOVE-05 | Move that pushes a descendant past depth 4 | `TooDeep`, checks the deepest descendant |
| T-MOVE-06 | A move is all or nothing | a failure leaves the tree unchanged |
| T-DEL-01 | Delete preview lists submodules and topics | preview matches the subtree |
| T-DEL-02 | Delete cascades the subtree | no orphans |
| T-DEL-03 | "Move contents up" re-parents children | children under the grandparent |
| T-DEL-04 | "Move contents up" reports clashes before deleting | clash list returned, nothing deleted |
| T-DEL-05 | "Move contents up" on a root | submodules become roots, own topics go to Default |
| T-DEL-06 | Deleting a topic leaves same-named topics | other "CPU" untouched |
| T-DEL-07 | Delete confirm needs the typed module name (component) | button disabled until the name matches |

## 4. Paths and breadcrumbs (core)

| ID | Test | Expect |
|---|---|---|
| T-PATH-01 | Full path built root to leaf | "Computer Architecture › N2T › CPU" |
| T-PATH-02 | A topic in a 4-deep module is shortened in the middle | first, "…", last two |
| T-PATH-03 | Path of a Default topic | "Default › Misc" |

## 5. Tree used elsewhere (core)

| ID | Test | Expect |
|---|---|---|
| T-USE-01 | Quiz on a module includes every descendant topic | union of the subtree |
| T-USE-02 | Unticked submodule excluded | none of its topics |
| T-USE-03 | Recall checklist about a module offers points from descendants | covered by T-USE-01's helper |
| T-USE-04 | "Tighten reviews" only touches topics inside the dated module | covered by T-USE-01's helper |
| T-USE-05 | Dashboard due count rolls up | parent total = children + own |
| T-USE-06 | Rollup doesn't double count | each item counted once per ancestor |

## 6. Related topics (core)

| ID | Test | Expect |
|---|---|---|
| T-REL-01 | Link is symmetric | both list each other |
| T-REL-02 | Linking doesn't merge | parents unchanged |
| T-REL-03 | Self link refused | `SelfLink` |
| T-REL-04 | Duplicate link ignored | one link stored |
| T-REL-05 | Quiz with "include related" | both topics |
| T-REL-06 | Related is one hop only | A–B–C, A doesn't bring C |
| T-REL-07 | Deleting a topic removes its links | no dangling links |
| T-REL-08 | Only topics can be related | `NotATopic` |

## 7. Search tokeniser (core)

| ID | Input | Expected tokens |
|---|---|---|
| T-TOK-01 | `CPU ALU` | Term(CPU), Term(ALU) |
| T-TOK-02 | `"CPU Logic"` | Phrase(CPU Logic) |
| T-TOK-03 | `"CPU Logic` | Phrase(CPU Logic), not closed |
| T-TOK-04 | `& $ ^ !` | And, Or, Xor, Not (symbols) |
| T-TOK-05 | `AND OR XOR NOT` | And, Or, Xor, Not (words) |
| T-TOK-06 | `and or` | Term(and), Term(or) |
| T-TOK-07 | `"AND"` | Phrase(AND) |
| T-TOK-08 | `!CPU` | Not, Term(CPU) |
| T-TOK-09 | `module:N2T` | Scope(module, N2T) |
| T-TOK-10 | `module:"UCM CySec CA Module"` | Scope(module, UCM CySec CA Module) |
| T-TOK-11 | `module:CS:APP` | Scope(module, CS:APP) |
| T-TOK-12 | `( )` | LParen, RParen |
| T-TOK-13 | `box$` | Term(box), Or |
| T-TOK-14 | extra spaces and tabs | ignored |
| T-TOK-15 | empty input | no tokens, empty query, no error |

## 8. Search parser and grouping (core)

| ID | Input | Tree |
|---|---|---|
| T-PAR-01 | `CPU ALU` | (CPU AND ALU) |
| T-PAR-02 | `N2T CPU $ ALU` | (N2T AND (CPU OR ALU)) |
| T-PAR-03 | `CPU & ALU $ RAM` | ((CPU AND ALU) OR RAM) |
| T-PAR-04 | `CPU & (ALU $ RAM)` | (CPU AND (ALU OR RAM)) |
| T-PAR-05 | `!(ALU $ RAM)` | NOT (ALU OR RAM) |
| T-PAR-06 | `!!CPU` | NOT NOT CPU |
| T-PAR-07 | `CPU ^ ALU ^ RAM` | ((CPU XOR ALU) XOR RAM) |
| T-PAR-08 | `a b $ c d` | ((a AND (b OR c)) AND d) |
| T-PAR-09 | `module:N2T $ module:CS:APP topic:CPU` | ((module:N2T OR module:CS:APP) AND topic:CPU) |
| T-PAR-10 | `CPU !module:CS:APP` | (CPU AND NOT module:CS:APP) |
| T-PAR-11 | `CPU & (ALU $ (RAM ^ ROM))` | (CPU AND (ALU OR (RAM XOR ROM))) |

| ID | Input | Error |
|---|---|---|
| T-PAR-E1 | `CPU $` | "$ needs something on its right" |
| T-PAR-E2 | `& ALU` | "& needs something on its left" |
| T-PAR-E3 | `CPU $ & ALU` | "Two operators in a row, remove one" |
| T-PAR-E4 | `(CPU $ ALU` | "Missing a closing bracket" |
| T-PAR-E5 | `CPU)` | "Extra closing bracket" |
| T-PAR-E6 | `!` | "! needs something to apply to" |
| T-PAR-E7 | `()` | "Empty brackets" |
| T-PAR-E8 | `CPU OR` | "OR needs something on its right" (the user's own spelling) |

## 9. Search evaluator (core)

| ID | Test | Expect |
|---|---|---|
| T-EVAL-01 | Contains, ignoring case | "cpu" matches "CPU", "CPUs", "the cpu cycle" |
| T-EVAL-02 | Phrase is exact and contiguous | "CPU Logic" matches "CPU Logic gates", not "Logic of a CPU" |
| T-EVAL-03 | AND truth table | only both |
| T-EVAL-04 | OR truth table | 3 of 4 |
| T-EVAL-05 | XOR truth table | exactly one |
| T-EVAL-06 | NOT | excludes matches |
| T-EVAL-07 | Plain term searches every kind | a hit of each kind |
| T-EVAL-08 | Matching topic brings its children | children listed under the topic |
| T-EVAL-09 | `module:N2T CPU` includes items in non-CPU topics | ALU question found |
| T-EVAL-10 | `module:N2T topic:CPU` limits to under that topic | ALU question not found |
| T-EVAL-11 | module scope includes descendants | CA finds N2T, CS:APP, UCM items |
| T-EVAL-12 | `topic:CPU` alone covers all same-named topics | both CPUs |
| T-EVAL-13 | Scope name exact, ignoring case | `module:n2t` works, `module:n2` nothing plus suggestion "N2T" |
| T-EVAL-14 | Two modules named N2T | both searched |
| T-EVAL-15 | Results grouped with counts | counts equal list lengths |
| T-EVAL-16 | Default topics are searchable | `module:Default` finds them |

## 10. Search properties (core, proptest)

| ID | Property |
|---|---|
| T-PROP-01 | parse(to_query(tree)) gives the same tree |
| T-PROP-02 | Symbol and keyword forms give identical trees |
| T-PROP-03 | NOT NOT x matches the same items as x |
| T-PROP-04 | x XOR y equals (x OR y) AND NOT (x AND y) |
| T-PROP-05 | Adding the brackets the display shows faint changes nothing |
| T-PROP-06 | Random input never panics |

## 11. Pill display model (core) and pills (component)

The core builds the display model the pills render, so its rules are tested in Rust and the component only draws it.

| ID | Test | Expect |
|---|---|---|
| T-PILL-01 | Space makes a pill (component) | pill made, input cleared |
| T-PILL-02 | Enter makes a pill and runs search (component) | search called |
| T-PILL-03 | Space inside an open quote (component) | no pill yet |
| T-PILL-04 | Enter with an unclosed quote (component) | one phrase pill |
| T-PILL-05 | Backspace on empty input (component) | last pill editable |
| T-PILL-06 | Remove by button and by Delete (component) | pill gone |
| T-PILL-07 | Paste a full query (component) | pills in one go |
| T-PILL-08 | Typing updates results, debounced (component) | one search per pause |
| T-PILL-09 | Symbols display as words (core) | `&` AND, `$` OR, `^` XOR, `!` NOT |
| T-PILL-10 | Space between terms displays as AND (core) | "CPU AND ALU" |
| T-PILL-11 | Operator merges its neighbours into one group (core) | one group |
| T-PILL-12 | AND group has no outline (core) | `Outline::None` |
| T-PILL-13 | OR solid, XOR dashed (core) | `Outline::Solid`, `Outline::Dashed` |
| T-PILL-14 | NOT greys and strikes through, also on groups (core) | negated flag |
| T-PILL-15 | Typed brackets solid, app-added faint (core) | `typed` flags |
| T-PILL-16 | Nested groups nest (core) | XOR group inside OR group |
| T-PILL-17 | Single term has no brackets (core) | none |
| T-PILL-18 | Scope pill shows icon, name and path tooltip (component) | all three |
| T-PILL-19 | Quoted "AND" stays a normal pill (core) | phrase pill |
| T-PILL-20 | Error outlines the offending pill (component) | aria-invalid, message |
| T-PILL-21 | Broken query keeps the last good results (component) | list unchanged |
| T-PILL-22 | "Edit as text" gives back what was typed (core) | exact string |
| T-PILL-23 | Scope autocomplete (core) | `module:N` lists N-modules |
| T-PILL-24 | Screen reader label (core) | "CPU or not ALU" |
| T-PILL-25 | State not by colour alone (component) | word labels, greyscale snapshot |
| T-PILL-26 | No results state (component) | message and clear button |
| T-PILL-27 | "?" opens the cheat sheet (component) | scopes and operators |

## 12. Graph view (component + e2e)

| ID | Test | Expect |
|---|---|---|
| T-GRAPH-01 | Nodes and parent edges match the tree | counts equal |
| T-GRAPH-02 | "Related" draws dashed edges | edges = links |
| T-GRAPH-03 | "Sources" adds source nodes | one per source |
| T-GRAPH-04 | Same-named nodes show parent on hover, path when selected | text correct |
| T-GRAPH-05 | Graph search highlights and dims | same matches as main search |
| T-GRAPH-06 | Double-click opens the topic | routed |
| T-GRAPH-07 | Arrow keys follow edges, Enter opens | focus order |
| T-GRAPH-08 | List fallback has the same nodes | tree matches |
| T-GRAPH-09 | 500 topics start collapsed | roots only, first render under 1s |
| T-GRAPH-10 | Empty state | message and button |

## 13. Data, sync, import and export (store)

| ID | Test | Expect |
|---|---|---|
| T-MIG-01 | Old flat modules become roots | topics unchanged |
| T-MIG-02 | Old module-less topics go to Default silently | moved |
| T-MIG-03 | Migration runs twice safely | no change |
| T-SYNC-01 | A move syncs | same parent on both |
| T-SYNC-02 | Offline move plus offline delete of the old parent | N2T restored as a root, notice |
| T-SYNC-03 | Two offline moves of one node | last write wins, notice |
| T-SYNC-04 | Sync can't create a cycle | node goes to roots, notice |
| T-IMP-01 | Anki `A::B::C` | modules A, B, topic C |
| T-IMP-02 | Anki 6 levels | past 4 joined into the topic name, reported |
| T-IMP-03 | Anki deck clashing with an existing topic | merged, reported |
| T-IMP-04 | Anki deck named "Default" | "Default (imported)" |
| T-EXP-01 | JSON round trip keeps tree, sources and links | equal |
| T-EXP-02 | Ids remapped on import | no clash |

## 14. End to end

| ID | Flow |
|---|---|
| E2E-01 | Create CA, three submodules, CPU in two of them, add a question to one, the other stays empty |
| E2E-02 | Topic with no module, warning, "No" then "Yes", found in Default |
| E2E-03 | `module:N2T topic:CPU` against `module:N2T CPU` |
| E2E-04 | Keyboard-only `CPU & (ALU $ (RAM ^ ROM))` reads "CPU AND (ALU OR (RAM XOR ROM))" |
| E2E-05 | Quiz on CA with CS:APP unticked has no CS:APP questions |
| E2E-06 | Graph search, open a topic by keyboard |
| E2E-07 | Delete with "Move contents up" and resolve a clash |

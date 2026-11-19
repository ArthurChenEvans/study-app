Feature: Nested modules and topics (R217 to R219)
  So that I can organise the same subject in several ways
  As a learner
  I want modules inside modules, with topics that belong to exactly one parent

  Background:
    Given a module "Computer Architecture"
    And it has submodules "N2T", "CS:APP" and "UCM CySec CA Module"

  # Structure

  Scenario: Create a submodule
    When I create a module "Extras" inside "Computer Architecture"
    Then "Extras" is shown under "Computer Architecture"

  Scenario: Same-named topics under different parents are separate
    When I create a topic "CPU" in "N2T"
    And I create a topic "CPU" in "CS:APP"
    Then there are two separate "CPU" topics
    And adding a question to "N2T › CPU" does not change "CS:APP › CPU"

  Scenario: Same-named topics under different parents keep their own sources
    Given "N2T › CPU" cites "The Elements of Computing Systems"
    And "CS:APP › CPU" cites "Computer Systems: A Programmer's Perspective"
    Then each topic lists only its own source

  Scenario: Duplicate name under the same parent is refused
    Given "N2T" has a topic "CPU"
    When I create another topic "CPU" in "N2T"
    Then I see "N2T already has a topic called CPU"
    And no topic is created

  Scenario: Names compare without case or outer spaces
    Given "N2T" has a topic "CPU"
    When I create a topic " cpu " in "N2T"
    Then it is refused as a duplicate

  Scenario: A topic can't contain modules
    When I try to create a module inside "N2T › CPU"
    Then the option is not offered

  Scenario Outline: Maximum nesting depth is 4 module levels
    Given a chain of modules <depth> levels deep
    When I create a submodule at the bottom
    Then the result is "<result>"

    Examples:
      | depth | result                                   |
      | 3     | created                                  |
      | 4     | refused, "Modules can go 4 levels deep" |

  # Default module

  Scenario: Creating a topic with no module asks first
    When I create a topic "Misc" without picking a module
    Then I see "Creating this topic without selecting a module will put this under Default. Are you sure you want this?"
    And below it "Default is not the fundamental module. It is a root module like all your other first layer modules. It is just where topics with unnamed modules go. Your root modules are not children of default"
    And the buttons are "Yes" and "No"

  Scenario: Confirming puts the topic in Default
    Given the Default warning is showing
    When I choose "Yes"
    Then "Misc" is created in "Default"

  Scenario: Declining returns to the form
    Given the Default warning is showing
    When I choose "No"
    Then I am back on the form with my typing kept and the module picker focused

  Scenario: Default is created only when first needed
    Given I have never put a topic in Default
    Then no "Default" module is shown

  Scenario: Default is a root module, not a parent of other roots
    Given "Default" exists
    Then "Default" and "Computer Architecture" are both shown at the top level
    And "Computer Architecture" is not inside "Default"

  Scenario: Topics can be moved out of Default
    When I move "Default › Misc" into "N2T"
    Then "Misc" is in "N2T"

  Scenario: Default can't be renamed or nested
    Then "Default" has no rename option and can't be moved inside another module

  Scenario: Deleting Default with topics in it
    Given "Default" has topics
    When I delete "Default"
    Then the confirmation lists its topics, the same as any other module

  Scenario: Submodules can't be created in Default
    Then "Default" offers no "New submodule" option

  Scenario: A user module named "Default" is refused
    When I create a module named "default"
    Then I see "Default is reserved for topics without a module"

  # Moving

  Scenario: Move a topic to another module
    When I move "N2T › CPU" into "UCM CySec CA Module"
    Then its questions, sources, schedule and history move with it

  Scenario: Moving into a parent that has the same name is blocked
    Given "UCM CySec CA Module" has a topic "CPU"
    When I move "N2T › CPU" into "UCM CySec CA Module"
    Then I am asked to rename it or cancel

  Scenario: A module can't be moved inside itself
    When I move "Computer Architecture" into "N2T"
    Then I see "A module can't go inside one of its own submodules"

  Scenario: A move past the depth limit is blocked
    Given a module with 2 levels of submodules
    When I move it 2 levels down
    Then the move is refused, with the depth limit stated

  # Deleting

  Scenario: Deleting a module names everything it removes
    When I delete "N2T"
    Then the confirmation lists its submodules, topics and question counts
    And nothing is deleted until I type the module name

  Scenario: Deleting a module can keep its contents
    When I delete "N2T" and choose "Move contents up"
    Then its topics and submodules move into "Computer Architecture"
    And any name clashes are listed so I can rename them first

  Scenario: Deleting a root module and moving its contents up
    When I delete "Computer Architecture" and choose "Move contents up"
    Then its submodules become root modules and its own topics go to "Default"

  Scenario: Deleting a topic doesn't touch same-named topics
    When I delete "N2T › CPU"
    Then "CS:APP › CPU" still exists

  # Showing which one is which

  Scenario: Results show the full path
    When I search "CPU"
    Then I see "Computer Architecture › N2T › CPU" and "Computer Architecture › CS:APP › CPU"

  Scenario: Long paths are shortened in the middle
    Given a topic 4 levels deep
    Then its breadcrumb shows the first and last two levels, with the full path in a tooltip

  # Using the tree elsewhere

  Scenario: Quizzing a module includes its submodules
    When I start a quiz on "Computer Architecture"
    Then questions from every topic in N2T, CS:APP and UCM CySec CA Module are included

  Scenario: Excluding a submodule from a quiz
    When I start a quiz on "Computer Architecture" and untick "CS:APP"
    Then no CS:APP questions are included

  Scenario: A free recall checklist about a module covers its submodules
    When I create a checklist about "Computer Architecture"
    Then I can import points from topics in any of its submodules

  Scenario: A calendar date covering a module covers its submodules
    Given a date "CA exam" covering "UCM CySec CA Module"
    When "tighten reviews" is ticked
    Then only topics inside "UCM CySec CA Module" are tightened

  Scenario: Dashboard rolls counts up the tree
    Then "Computer Architecture" shows the total due across its submodules

  # Related topics (optional link, never a merge) (R218)

  Scenario: Link related topics
    When I mark "N2T › CPU" as related to "CS:APP › CPU"
    Then both topics show the link
    And their questions and sources stay separate

  Scenario: A quiz can include related topics
    Given "N2T › CPU" is related to "CS:APP › CPU"
    When I quiz "N2T › CPU" with "Include related topics" ticked
    Then questions from both are included

  Scenario: A topic can't be related to itself
    When I relate "N2T › CPU" to "N2T › CPU"
    Then the option is not offered

  # Graph view (R219)

  Scenario: Graph shows the tree
    When I open the graph view
    Then modules and topics are nodes joined by parent links

  Scenario: Graph shows related links and sources
    When I turn on "Related" and "Sources" in the graph
    Then related links are drawn dashed and sources appear as their own nodes

  Scenario: Same-named topics are told apart in the graph
    Then each "CPU" node shows its parent on hover and its full path when selected

  Scenario: Graph search uses the same query language
    When I search the graph for "module:N2T CPU $ ALU"
    Then matching nodes are highlighted and the rest are dimmed

  Scenario: Open a topic from the graph
    When I double-click "N2T › CPU" in the graph
    Then that topic opens

  Scenario: Graph is usable by keyboard
    When I focus the graph and press the arrow keys
    Then focus moves between linked nodes and Enter opens the focused one

  Scenario: Graph has a list fallback
    When I choose "Show as list"
    Then the same nodes appear as an indented tree

  Scenario: Large graphs stay readable
    Given 500 topics
    When I open the graph view
    Then modules start collapsed and expand when clicked

  Scenario: Empty graph
    Given no modules
    When I open the graph view
    Then I see "No modules yet" and a "Create a module" button

  # Existing data, sync and import/export

  Scenario: Existing flat modules upgrade without changes
    Given data from before nesting
    When the app updates
    Then every existing module becomes a root module with its topics unchanged

  Scenario: Existing topics with no module go to Default
    Given a topic from before nesting with no module
    When the app updates
    Then it is in "Default" and no warning is shown

  Scenario: Moves sync across devices
    Given I move "N2T › CPU" on my laptop
    When my other device syncs
    Then the topic appears in its new place there

  Scenario: Conflicting moves while offline
    Given I move "N2T" on one device and delete "Computer Architecture" on another while offline
    When both sync
    Then nothing is lost, and "N2T" is restored as a root module with a notice

  Scenario: Anki nested decks import as nested modules
    When I import an Anki deck named "Computer Architecture::N2T::CPU"
    Then I get the modules "Computer Architecture" and "N2T" and the topic "CPU"

  Scenario: Anki decks deeper than the limit
    When I import an Anki deck nested 6 levels deep
    Then the levels past 4 are joined into the topic name, and I am told which decks were flattened

  Scenario: Export keeps the tree
    When I export "Computer Architecture" as JSON
    Then importing that file rebuilds the same tree, sources and related links

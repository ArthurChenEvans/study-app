Feature: Search query language (R220)
  So that I can find exactly what I want without wading through lists
  As a learner
  I want scoped search with AND, OR, NOT and XOR, written as pills

  Background:
    Given "N2T" has a topic "CPU" with questions, flashcards and a recall checklist
    And "CS:APP" has a topic "CPU"
    And "N2T" has a topic "ALU" with a question mentioning "CPU"

  # Plain search

  Scenario: A plain word searches everything
    When I search "CPU"
    Then I see matching modules, topics, quiz questions, flashcards, recall points and practice log entries from every module

  Scenario: A matching topic shows its children
    When I search "CPU"
    Then each "CPU" topic is listed with its questions, cards and checklists nested under it

  Scenario: Matching is by "contains", ignoring case
    When I search "cpu"
    Then "CPU", "CPUs" and "the cpu cycle" all match

  Scenario: Results are grouped by kind
    Then results are grouped into Modules, Topics, Questions, Flashcards, Free Recall and Practice Log, each with a count

  # Scopes

  Scenario: module: narrows to anything inside a module
    When I search "module:N2T CPU"
    Then I see everything inside N2T containing "CPU", including the ALU question
    And nothing from CS:APP

  Scenario: topic: narrows to things under a topic
    When I search "module:N2T topic:CPU"
    Then I see only the questions, cards and checklists under "N2T › CPU"
    And the ALU question is not shown

  Scenario: module: includes submodules
    When I search "module:\"Computer Architecture\" CPU"
    Then matches from N2T, CS:APP and UCM CySec CA Module are shown

  Scenario: topic: on its own covers every topic with that name
    When I search "topic:CPU"
    Then items under both "N2T › CPU" and "CS:APP › CPU" are shown

  Scenario: Scope names match exactly, ignoring case
    When I search "module:n2"
    Then I see "No module called n2", with "N2T" offered as a suggestion

  Scenario: Scope names with spaces need quotes
    When I search "module:\"UCM CySec CA Module\""
    Then it is read as one module name

  Scenario: Two modules with the same name
    Given "N2T" also exists under "Hobby"
    When I search "module:N2T"
    Then both are searched and results show their full paths

  Scenario: Scope suggestions while typing
    When I type "module:N"
    Then a list suggests module names starting with "N", with their paths

  # Pills

  Scenario: Space turns a term into a pill
    When I type "CPU" then press space
    Then "CPU" becomes a pill and the cursor moves on

  Scenario: Enter turns the last term into a pill and runs the search
    When I type "CPU" then press Enter
    Then "CPU" becomes a pill and results update

  Scenario: Quotes keep words together
    When I type "\"CPU Logic\"" then press space
    Then one pill "CPU Logic" is made, which matches that exact phrase

  Scenario: Spaces inside an open quote don't make a pill
    When I type "\"CPU Lo" then press space
    Then no pill is made yet

  Scenario: An unclosed quote on Enter
    When I type "\"CPU Logic" then press Enter
    Then the quote is closed for me and one pill "CPU Logic" is made

  Scenario: Unquoted words are separate pills joined by AND
    When I type "CPU Logic" and press space
    Then I get pills "CPU" and "Logic", which match items containing both

  Scenario: Backspace on an empty input edits the last pill
    Given pills "CPU" and "ALU"
    When I press Backspace with nothing typed
    Then "ALU" goes back to text I can edit

  Scenario: Pills can be removed by click or keyboard
    When I click a pill's remove button, or focus it and press Delete
    Then that pill is removed and results update

  Scenario: Pasting a full query
    When I paste "module:N2T CPU $ ALU !draft"
    Then it is turned into the matching pills in one go

  Scenario: Searches update as I type
    Then results update while I type, not only on Enter

  # Operators

  Scenario Outline: Symbols and capital keywords both work
    When I search "<query>"
    Then it means "<meaning>"

    Examples:
      | query        | meaning           |
      | CPU & ALU    | CPU AND ALU       |
      | CPU AND ALU  | CPU AND ALU       |
      | CPU $ ALU    | CPU OR ALU        |
      | CPU OR ALU   | CPU OR ALU        |
      | CPU ^ ALU    | CPU XOR ALU       |
      | CPU XOR ALU  | CPU XOR ALU       |
      | !CPU         | NOT CPU           |
      | NOT CPU      | NOT CPU           |

  Scenario: Lower-case keywords are plain words
    When I search "cpu and alu"
    Then it searches for "cpu", "and" and "alu" all together

  Scenario: Searching for an operator word itself
    When I search "\"AND\""
    Then it matches items containing the word "AND"

  Scenario: XOR means one or the other, not both
    Given item A contains "CPU", item B contains "ALU" and item C contains both
    When I search "CPU ^ ALU"
    Then A and B are shown and C is not

  Scenario: An operator joins only its immediate neighbours
    When I search "N2T CPU $ ALU"
    Then it means N2T AND (CPU OR ALU)

  Scenario: A chain of explicit operators reads left to right
    When I search "CPU & ALU $ RAM"
    Then it means (CPU AND ALU) OR RAM

  Scenario: Brackets group terms
    When I search "CPU & (ALU $ RAM)"
    Then it means CPU AND (ALU OR RAM)

  Scenario: NOT works on a group
    When I search "!(ALU $ RAM)"
    Then items containing neither ALU nor RAM are shown

  Scenario: NOT works on a scope
    When I search "CPU !module:CS:APP"
    Then CPU matches from everywhere except CS:APP are shown

  Scenario: Operators work across scopes
    When I search "module:N2T $ module:CS:APP topic:CPU"
    Then items under a "CPU" topic in either N2T or CS:APP are shown

  Scenario Outline: Broken queries explain themselves
    When I search "<query>"
    Then the problem pill is outlined and I see "<message>"

    Examples:
      | query        | message                                    |
      | CPU $        | "$ needs something on its right"           |
      | & ALU        | "& needs something on its left"            |
      | CPU $ & ALU  | "Two operators in a row, remove one"       |
      | (CPU $ ALU   | "Missing a closing bracket"                |
      | !            | "! needs something to apply to"            |

  Scenario: A broken query keeps the last good results
    Given results are showing for "CPU"
    When I type "CPU $"
    Then the "CPU" results stay until the query is fixed

  # How pills look (never colour alone)

  Scenario Outline: Operator symbols become words
    When I type "<typed>"
    Then the pills read "<shown>"

    Examples:
      | typed      | shown          |
      | CPU & ALU  | CPU AND ALU    |
      | CPU $ ALU  | CPU OR ALU     |
      | CPU ^ ALU  | CPU XOR ALU    |
      | !CPU       | NOT CPU        |

  Scenario: Typed keywords look the same as symbols
    When I type "CPU OR ALU"
    Then it looks exactly like "CPU $ ALU"

  Scenario: The space between terms shows as AND
    When I type "CPU ALU"
    Then the pills read "CPU AND ALU"

  Scenario: An operator merges with the pills it joins
    When I type "CPU $ ALU"
    Then "CPU", "OR" and "ALU" become one joined pill group
    And the operator word sits between the terms in a smaller, muted style

  Scenario: AND groups are plain
    When I type "CPU & ALU"
    Then the group has no outline

  Scenario: OR groups have a solid outline
    When I type "CPU $ ALU"
    Then the group sits inside a solid blue outline

  Scenario: XOR groups have a dashed outline
    When I type "CPU ^ ALU"
    Then the group sits inside a dashed outline

  Scenario: NOT is greyed and struck through
    When I type "!CPU"
    Then the pill reads "NOT CPU", with "CPU" greyed out and struck through

  Scenario: NOT on a group strikes through the whole group
    When I type "!(ALU $ RAM)"
    Then it reads "NOT (ALU OR RAM)" and the whole group is greyed and struck through

  Scenario: Typed brackets are shown
    When I type "CPU & (ALU $ RAM)"
    Then it reads "CPU AND (ALU OR RAM)", with the brackets drawn around the inner group

  Scenario: Grouping the app works out is shown as faint brackets
    When I type "N2T CPU $ ALU"
    Then it reads "N2T AND (CPU OR ALU)"
    And the brackets are faint, to show I didn't type them

  Scenario: Left-to-right chains show their grouping
    When I type "CPU & ALU $ RAM"
    Then it reads "(CPU AND ALU) OR RAM", with faint brackets

  Scenario: Nested groups nest visually
    When I type "CPU & (ALU $ (RAM ^ ROM))"
    Then it reads "CPU AND (ALU OR (RAM XOR ROM))"
    And the dashed XOR outline sits inside the solid OR outline

  Scenario: A single term gets no brackets
    When I type "CPU"
    Then no brackets or outlines are shown

  Scenario: Scope pills show their kind
    When I type "module:N2T"
    Then the pill shows a module icon and "N2T", with the full path in a tooltip

  Scenario: Pills read correctly to screen readers
    When I focus the pill group for "CPU $ !ALU"
    Then it is read as "CPU or not ALU"

  Scenario: The query can be edited as text
    When I choose "Edit as text"
    Then the pills turn back into what I typed, keeping my symbols or keywords

  # Where search is used

  Scenario: The same language works across the app
    Then the main search, list filters on every screen and graph search all accept the same queries

  Scenario: Help is one click away
    When I click the "?" next to the search box
    Then a short cheat sheet shows the scopes, operators and an example of each

  Scenario: No results
    When I search "zzz"
    Then I see "Nothing matches zzz", with a button to clear the search

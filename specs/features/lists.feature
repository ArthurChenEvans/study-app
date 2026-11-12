Feature: Lists at scale

  # R138
  Scenario: Narrow a long list by module
    Given 171 topics in 14 modules
    When I pick "Networks" in the module rail
    Then only Networks topics are listed
    And the rail shows how many topics each module has

  # R139
  Scenario: Filter as you type
    When I press "/" and type "cache"
    Then the list shows only topics whose name, module or tag has "cache", with it marked
    And it says "9 of 171"
    When I press Esc
    Then every topic is listed again

  # R139
  Scenario: Nothing matches
    When I filter by "kerberos"
    Then it says nothing matches "kerberos"
    And offers to clear the filter or search inside questions

  # R140
  Scenario: Only what's due
    When I pick "Due"
    Then only topics whose quiz is due are listed, with the count on the button

  # R141
  Scenario: Sort and pick columns
    When I click the "Next quiz" header
    Then topics are sorted soonest first, and the header says ascending
    When I click it again
    Then they are sorted the other way
    When I turn on the "Last score" column
    Then it's still on next time

  # R142
  Scenario: The page stays one window tall
    Given 171 topics
    Then the list scrolls in its own pane and the header row stays in view

  # R143
  Scenario: The item card
    When I click "Subnetting"
    Then its details and actions show beside the list
    When I press the down arrow
    Then the next topic is picked
    When I press Enter
    Then it opens to edit

  # R144
  Scenario: Act on ticked topics
    When I tick three topics
    Then a bar says "3 ticked" with Quiz them, Move to module and Tag
    When I move them to "Networks"
    Then all three are in Networks

  # R145
  Scenario: Pick a whole module for a quiz
    Given I'm making a new quiz
    When I pick "Networks" in the rail and press "Tick all 5 in Networks"
    Then Your quiz lists the five topics as chips
    When I remove one chip
    Then that topic is unticked

  # R146
  Scenario: Quizzes are capped a day
    Given 66 topic quizzes are overdue and the limit is 5
    Then 5 are due today, most overdue first
    And Today says "61 more waiting" and when the last of them comes in
    And a waiting topic keeps its step

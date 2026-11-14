Feature: Projects, struggles and drills
  Applied work isn't scheduled. Struggles are logged while working, reviewed at the end of
  the session, and can become cards, checklist points or drills.

  # R87, R88
  Scenario: Work on a project in sessions
    Given a project "Study app" of kind "Coding"
    When I start a session
    Then the session timer runs
    And I can't start a second session on "Study app"
    When I end the session
    Then the session is saved with how long it took

  # R89
  Scenario: Log a struggle during a session
    Given a session running on "Study app"
    When I log "Loop ran one too many times" because "I used <= out of habit" as couldn't apply
    Then it's listed under today with the time it was logged, to review

  # R90, R95
  Scenario: A recall miss becomes a card suggestion
    Given a session with the struggle "Forgot how to slice a string from the end" as couldn't recall
    When I end the session and choose "Suggest a card" for it
    Then Suggestions has a card starting with the front "Forgot how to slice a string from the end"
    And it shows why it wasn't obvious

  # R90
  Scenario: A struggle becomes a recall checklist point
    Given a checklist about "Rust"
    When I review a struggle and add it to that checklist
    Then the checklist has the struggle as a point

  # R91, R92
  Scenario: A struggle with a new issue gets a new drill
    Given a session with the struggle "Couldn't borrow inside a closure" as couldn't apply
    When I review it with the new issue "Borrowing in closures" and write a drill
    Then the drill is scheduled for tomorrow unless I pick another day
    And the issue "Borrowing in closures" exists for every project

  # R91
  Scenario: A known issue offers its drill again
    Given the issue "Off-by-one in loops" has a drill that was ticked off
    When a later review gives a struggle the issue "off-by-one in loops"
    Then I'm offered to schedule that drill again or write a different one

  # R93
  Scenario Outline: Rating a drill
    Given a drill due today
    When I rate it <rating>
    Then it is <result>

    Examples:
      | rating    | result               |
      | Struggled | due again tomorrow   |
      | Shaky     | due again tomorrow   |
      | Solid     | ticked off           |

  # R90
  Scenario: Finish a review later
    Given a session with 3 struggles
    When I review 1 and choose "Finish later"
    Then the project shows 2 struggles to review

  # R190
  Scenario: A kind of its own, nested
    When I make a practice log and type the kind "Coding::Rust"
    Then the Coding filter shows it with my other Coding logs

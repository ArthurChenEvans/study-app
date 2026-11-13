Feature: Celebrations
  Big moments get a full-screen celebration and small milestones pop up along the way,
  so finishing feels good. Everything can be tuned or turned off.

  # R154
  Scenario: Finishing today's reviews
    Given I have 64 cards due
    When I grade the last one
    Then a full-screen "Reviews done" moment shows cards, minutes and how many I remembered
    And Continue closes it

  Scenario: A quiz celebrates in proportion to the score
    When I finish a quiz with 100%
    Then the badge is gold, the jump is highest and there are three rounds of confetti
    When I finish a quiz with 58%
    Then there is a small pop of confetti and a gentler chord

  Scenario: A stray press doesn't skip it
    When a full-screen moment opens
    And I press Continue straight away
    Then it stays open until half a second has gone

  # R151, R153
  Scenario Outline: Small pops scale with the queue
    Given a session of <queue> cards
    Then pops land at <where>

    Examples:
      | queue | where                                   |
      | 8     | nowhere, the finish screen is enough    |
      | 30    | 15 (Halfway)                            |
      | 64    | 15, 32 (Halfway), 48 (Last stretch)     |
      | 120   | 30, 60 (Halfway), 90 (Last stretch)     |

  Scenario: Streaks
    When I get 5 right in a row
    Then "5 in a row!" pops up
    When I get one wrong
    Then the streak starts again

  # R152
  Scenario: Turn everything off
    When I turn off Celebrations
    Then no pops or full-screen moments show
    But the end-of-session summary still shows

  Scenario: Shrink one moment
    When I set Quiz complete to Small pop
    Then finishing a quiz shows a pop in the corner instead of the full screen

  # R150
  Scenario: Choices follow me, sound doesn't
    When I turn off answer streaks on my desktop
    Then they're off on my laptop after it syncs
    But my laptop keeps its own volume and motion

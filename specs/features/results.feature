@results
Feature: Review results
  As a student
  I want to see how I did and why I got things wrong
  So that I know what to revise

  Background:
    Given a topic "CPU" with 6 questions
    And a topic "GPU" with 4 questions
    And I finished a quiz on "CPU" and "GPU" with 8 right, 2 wrong and 2 of the right ones flagged as guessed

  Scenario: Results summary
    When I see the results
    Then the score is 8 out of 10
    And I see that 2 were guessed
    And I see a breakdown for "CPU" and for "GPU"

  Scenario: Review each question
    When I review the results
    Then I see every question with my answer, the correct answer and the explanation

  Scenario: Give a reason for a wrong answer
    When I give the reason "confused with something similar" to a wrong answer
    Then the reason is saved with that answer

  Scenario: Give a free text reason
    When I give the reason "mixed up L1 and L2 sizes" as free text to a wrong answer
    Then the reason is saved with that answer

  Scenario: Skip the reason
    When I see the results
    Then I am prompted for a reason on each wrong answer
    And I can leave the results without giving one

  Scenario Outline: Automatic reasons
    Given a wrong answer that was <cause>
    When I see the results
    Then that answer is already recorded as <reason>

    Examples:
      | cause                                  | reason     |
      | missing some correct options           | incomplete |
      | not answered before the timer ran out  | timed out  |

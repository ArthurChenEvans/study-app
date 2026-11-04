@retry
Feature: Retry wrong and guessed questions
  As a student
  I want to redo the questions I got wrong or only guessed
  So that I practise exactly what I don't know yet

  Background:
    Given a topic "CPU" with 10 questions

  Scenario: Retry wrong and guessed
    Given I finished a quiz on "CPU" with 2 wrong and 1 right but flagged as guessed
    When I retry
    Then the retry has those 3 questions

  Scenario: Retry uses same settings
    Given I finished an exam mode quiz on "CPU" with a 30 second per-question timer and shuffled options
    And I got some questions wrong
    When I retry
    Then the retry is in exam mode with a 30 second per-question timer and shuffled options

  Scenario: Retry is saved as a retry
    Given I finished a quiz on "CPU" with 2 wrong
    When I retry and finish
    Then the retry appears in history labelled as a retry

  @edge
  Scenario: Nothing to retry
    Given I finished a quiz on "CPU" with every answer right and none guessed
    Then there is nothing to retry

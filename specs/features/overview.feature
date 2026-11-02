@overview
Feature: Overview of progress
  As a student
  I want to see how I'm doing across all my quizzes
  So that I know whether I'm improving and what to revise next

  Background:
    Given these finished attempts
      | date   | topics           | right | total | guessed | retry |
      | 20 Oct | CPU, GPU         | 5     | 6     | 1       | yes   |
      | 22 Oct | GPU              | 11    | 18    | 3       | no    |
      | 26 Oct | CPU, GPU         | 13    | 20    | 2       | no    |
      | 29 Oct | OSI model        | 4     | 15    | 1       | no    |
      | 1 Nov  | CPU, GPU         | 16    | 20    | 3       | no    |
    And today is 1 Nov

  Scenario: Progress summary
    When I open the overview
    Then I see my accuracy over the last 7 days
    And I see how many questions I answered in the last 7 days
    And I see what share of my right answers were guessed

  Scenario: Change from the previous week
    Given my accuracy over the last 7 days is 63%
    And my accuracy over the 7 days before was 59%
    When I open the overview
    Then I see a change of "+4 points"

  Scenario: Score per quiz
    When I open the overview
    Then I see each attempt's score as a percentage, in date order

  Scenario: Retries on the chart
    When I open the overview
    Then the 20 Oct attempt is marked as a retry on the score chart

  Scenario: Weakest topics first
    Given the latest quiz on "OSI model" scored 27%
    And the latest quiz on "CPU" scored 85%
    When I open the overview
    Then "OSI model" is listed above "CPU"
    And each topic shows how many of its answers were right and sure, right but guessed, and wrong

  Scenario: Topics never quizzed
    Given a topic "Misc" that has never been quizzed
    When I open the overview
    Then "Misc" is noted as not quizzed yet

  Scenario: Reasons ranked
    Given wrong answers in the last 30 days with the reasons
      | reason                          | count |
      | Confused with something similar | 14    |
      | Misremembered                   | 11    |
      | Blank                           | 7     |
    When I open the overview
    Then the reasons are listed from most to least common with their counts

  @edge
  Scenario: First run
    Given I have no attempts
    When I open the overview
    Then I see first steps to create a topic, write questions and take a quiz
    And I don't see any progress charts

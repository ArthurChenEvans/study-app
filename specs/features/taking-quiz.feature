@quiz
Feature: Take a quiz
  As a student
  I want to answer questions in study or exam mode
  So that I can either learn as I go or test myself properly

  Background:
    Given a topic "CPU" with 10 questions

  @study
  Scenario: Study mode feedback
    Given I started a study mode quiz on "CPU"
    When I answer the first question
    Then I see whether I was right before the next question

  @exam
  Scenario: Exam mode feedback
    Given I started an exam mode quiz on "CPU"
    When I answer the first question
    Then I don't see whether I was right
    And I only see results after the last question

  @exam @edge
  Scenario: No going back in exam mode
    Given I started an exam mode quiz on "CPU"
    And I answered the first question
    When I try to go back to the first question
    Then I can't change my answer

  Scenario Outline: Easy and hard mode
    Given a multiple-answer question with 2 correct options in "CPU"
    And I started a quiz on "CPU" in <difficulty> mode
    When the question is shown
    Then the number of correct answers is <shown>

    Examples:
      | difficulty | shown  |
      | easy       | shown  |
      | hard       | hidden |

  Scenario Outline: Flag a question as guessed
    Given I started a <mode> mode quiz on "CPU"
    When I flag the first question as guessed
    And I answer it
    Then the question is recorded as guessed

    @study
    Examples:
      | mode  |
      | study |

    @exam
    Examples:
      | mode |
      | exam |

  Scenario Outline: All or nothing scoring
    Given a multiple-answer question with the correct options "A" and "C"
    When I answer it with <picked>
    Then it is marked <result>

    Examples:
      | picked         | result |
      | "A" and "C"    | right  |
      | "A" only       | wrong  |
      | "A", "B", "C"  | wrong  |
      | "B" only       | wrong  |

  @edge
  Scenario: Per-question timeout
    Given I started a quiz on "CPU" with a 30 second per-question timer
    When 30 seconds pass without an answer
    Then the question is counted as wrong
    And the next question is shown

  @edge
  Scenario: Total timeout
    Given I started a quiz on "CPU" with a 5 minute total timer
    And I answered 6 questions
    When 5 minutes pass
    Then the quiz ends
    And I see the results

  @edge
  Scenario: Quit halfway
    Given I started a quiz on "CPU"
    And I answered 4 questions
    When I quit the quiz
    Then the attempt is saved as incomplete
    And it appears in history

  Scenario Outline: Keyboard shortcuts
    Given I am answering a question in a quiz
    When I press <key>
    Then <effect>

    Examples:
      | key   | effect                         |
      | 1     | the first option is picked     |
      | 5     | the fifth option is picked     |
      | Enter | my answer is submitted         |
      | F     | the question is flagged as guessed |

  # Added by the November 2026 robustness audit
  Scenario: A quiz in progress survives the app closing
    Given I have answered 2 of 4 questions in a quiz
    When the app closes, crashes or the computer loses power
    And I open the app again
    Then the quiz opens at question 3 with my 2 answers kept
    And it is not in history until I finish or quit it

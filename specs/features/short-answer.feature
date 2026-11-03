@short-answer
Feature: Short answer questions
  As a student
  I want to type answers and have them checked
  So that I practise recalling, not just recognising

  Background:
    Given a topic "OSI model"

  # R78, R79
  Scenario: A short answer question needs something to check against
    When I write a short answer question with no accepted answers
    Then it isn't saved and I'm told "add at least one accepted answer, so the app can check it"

  Scenario: Switching type keeps what's shared
    Given a multiple choice question with an explanation, a source and tags
    When I switch it to short answer
    Then the question, explanation, source and tags are kept

  # R80
  Scenario Outline: Accepted answers are forgiving
    Given a short answer question accepting "Transport layer" with small typos allowed
    When I answer "<typed>"
    Then it is marked <verdict>

    Examples:
      | typed                | verdict              |
      | transport layer      | right                |
      | The Transport Layer. | right                |
      | the transprot layer  | right, with a typo   |
      | the transprto layre  | wrong                |
      | network layer        | wrong                |

  Scenario: No typos in numbers or short words
    Given a short answer question accepting "443" and one accepting "TCP"
    When I answer "433" and "TPC"
    Then both are wrong

  # R81
  Scenario Outline: Keywords in any order, each word counted once
    Given a short answer question with keywords "SYN", "SYN-ACK" (also "synack") and "ACK", all needed
    When I answer "<typed>"
    Then it is marked <verdict> with <hits> of 3 keywords

    Examples:
      | typed                              | verdict | hits |
      | SYN, SYN-ACK, ACK                  | right   | 3    |
      | ack then synack then syn           | right   | 3    |
      | SYN, then the server sends SYN-ACK | wrong   | 2    |

  # R82
  Scenario: Try an answer before saving
    When I try "the transprot layer" in the editor
    Then I see "Right, with a typo" and nothing is recorded

  # R83
  Scenario: Study mode shows the verdict
    Given a study quiz with that question
    When I type "network layer" and press Enter
    Then I see "Wrong", the model answer and the explanation

  # R84
  Scenario: Overrule a verdict
    Given I answered "network layer" and it was marked wrong
    When I count it as right
    Then the results, history and Overview count it as right and it's labelled as overruled
    And it isn't suggested as a card or retried
    When I undo that
    Then it counts as wrong again

  # R85
  Scenario: Choose the question types for a quiz
    Given "OSI model" has 4 MCQs and 3 short answer questions
    When I start a quiz with only short answers
    Then it has 3 questions, all short answer

  # R86
  Scenario: Short answers feed recall checklists and cards
    Given I got a short answer question wrong
    Then it is suggested as a card starting with the question and the model answer
    And it can be added to a checklist as "question → model answer"

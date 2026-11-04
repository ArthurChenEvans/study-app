@questions
Feature: Write multiple choice questions
  As a student
  I want to write MCQs with rich content
  So that my questions match how the material is really written

  Background:
    Given a topic called "CPU"

  Scenario: Write a question with an explanation
    When I write a question in "CPU" with the text "What does the ALU do?"
    And I give it the options "Arithmetic and logic", "Stores data", "Fetches instructions"
    And I mark "Arithmetic and logic" as correct
    And I give it the explanation "The ALU performs arithmetic and logic operations."
    And I save the question
    Then "CPU" has 1 question
    And the question shows its explanation

  Scenario: A question belongs to one topic
    Given a topic called "GPU"
    When I save a question in "CPU"
    Then the question is in "CPU"
    And the question is not in "GPU"

  Scenario Outline: Option count limits
    When I write a question in "CPU" with <count> options and one marked correct
    And I save the question
    Then the save is <result>

    Examples:
      | count | result   |
      | 2     | refused  |
      | 3     | accepted |
      | 4     | accepted |
      | 5     | accepted |
      | 6     | refused  |

  @edge
  Scenario: No correct option
    When I write a question in "CPU" with 4 options and none marked correct
    And I save the question
    Then the save is refused
    And I am told to mark at least one correct option

  Scenario: Single-answer question
    When I write a single-answer question in "CPU" with 4 options
    And I mark one option as correct
    And I save the question
    Then the question has exactly 1 correct option

  Scenario: Multiple-answer question
    When I write a multiple-answer question in "CPU" with 5 options
    And I mark 3 options as correct
    And I save the question
    Then the question has 3 correct options

  Scenario Outline: Rich content in every part
    When I write a question in "CPU" with <content> in the <part>
    And I save the question
    Then the <part> shows <content> rendered

    Examples:
      | part        | content                     |
      | question    | a highlighted code block    |
      | question    | a LaTeX formula             |
      | question    | an image                    |
      | option      | a highlighted code block    |
      | option      | a LaTeX formula             |
      | option      | an image                    |
      | explanation | a highlighted code block    |
      | explanation | a LaTeX formula             |
      | explanation | an image                    |

  Scenario Outline: Add an image by pasting or picking
    When I add an image to a question by <method>
    And I save the question
    Then the question shows the image
    And the image is kept with the question

    Examples:
      | method               |
      | pasting it           |
      | picking it from file |

  Scenario: Tag a question
    Given a question in "CPU"
    When I tag the question with "registers"
    Then the question has the tag "registers"

  @edge
  Scenario: Delete a question that is in history
    Given a question in "CPU"
    And a finished attempt that included the question
    When I delete the question and confirm
    Then the question no longer appears in "CPU"
    And the attempt in history still shows the question as it was

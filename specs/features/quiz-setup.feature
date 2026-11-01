@quiz @setup
Feature: Set up a quiz
  As a student
  I want to choose what and how I'm quizzed before starting
  So that each quiz fits what I'm revising

  Background:
    Given a topic "CPU" with 10 questions
    And a topic "GPU" with 10 questions

  Scenario: Quiz on several topics
    When I set up a quiz on "CPU" and "GPU" with all questions
    And I start the quiz
    Then the quiz has 20 questions
    And it includes questions from both "CPU" and "GPU"

  Scenario Outline: All questions or a set number
    When I set up a quiz on "CPU" and "GPU" with <amount>
    And I start the quiz
    Then the quiz has <count> questions

    Examples:
      | amount       | count |
      | all questions | 20   |
      | 8 questions   | 8    |

  Scenario: Random pick across topics
    When I set up a quiz on "CPU" and "GPU" with 8 questions picked randomly
    And I start the quiz
    Then the quiz has 8 questions drawn at random from both topics combined

  Scenario: Even split across topics
    When I set up a quiz on "CPU" and "GPU" with 8 questions picked evenly
    And I start the quiz
    Then the quiz has 4 questions from "CPU" and 4 from "GPU"

  Scenario Outline: Order options
    When I set up a quiz with the question order <questions> and the option order <options>
    And I start the quiz
    Then the questions are <questions>
    And the options within each question are <options>

    Examples:
      | questions | options  |
      | kept      | kept     |
      | kept      | shuffled |
      | shuffled  | kept     |
      | shuffled  | shuffled |

  Scenario Outline: Timers are optional
    When I set up a quiz with <total> total timer and <per> per-question timer
    And I start the quiz
    Then the quiz runs with <total> total timer and <per> per-question timer

    Examples:
      | total | per |
      | no    | no  |
      | a     | no  |
      | no    | a   |
      | a     | a   |

  Scenario: Hard mode is the default
    When I set up a quiz without choosing easy or hard
    Then the quiz is set to hard

@history
Feature: Quiz history
  As a student
  I want my past attempts kept
  So that I can look back at how I've done

  Background:
    Given a topic "CPU" with 10 questions

  Scenario: Attempts listed in history
    Given I finished 2 quizzes on "CPU"
    When I open history
    Then I see both attempts with their scores

  @edge
  Scenario: History shows the question as it was
    Given I finished a quiz that included the question "What does the ALU do?"
    When I change that question's text to "What is the ALU's job?"
    And I look at the attempt in history
    Then it shows "What does the ALU do?"

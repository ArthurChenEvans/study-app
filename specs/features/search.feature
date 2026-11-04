@search
Feature: Search
  As a student
  I want fast, filterable search across everything
  So that I can find any question, topic or source quickly

  Background:
    Given a module "Computer Architecture" with the topics "CPU" and "GPU"
    And a question in "GPU" with the text "What is a shader core?" and the tag "hardware"

  Scenario: Search as you type
    When I type "shad" into search
    Then "What is a shader core?" is in the results without me submitting

  Scenario Outline: Search question content
    Given <part> contains the word "pipeline"
    When I search for "pipeline"
    Then that question is in the results

    Examples:
      | part                  |
      | a question's text     |
      | an option             |
      | an explanation        |

  Scenario Outline: Search across categories
    Given <thing> contains the word "pipeline"
    When I search for "pipeline"
    Then the result appears under the <category> tab

    Examples:
      | thing          | category |
      | a topic name   | topics   |
      | a tag          | tags     |
      | a source title | sources  |
      | a module name  | modules  |

  Scenario: Combine filters
    Given a question in "CPU" with the tag "hardware"
    And a topic "Networking" outside any module with a question tagged "hardware"
    When I filter by the tag "hardware" and the module "Computer Architecture"
    Then I see the "GPU" and "CPU" questions
    And I don't see the "Networking" question

  Scenario: Sort results
    Given several results for a search
    When I sort the results by a column
    Then the results are ordered by that column

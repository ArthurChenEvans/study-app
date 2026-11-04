@sources
Feature: Read sources and cite them
  As a student
  I want to read my lecture files beside the editor and cite where facts came from
  So that every question is backed by a source I can check

  Background:
    Given a topic called "CPU"

  Scenario Outline: Open a file beside the editor
    When I open a <format> file while writing questions in "CPU"
    Then the file is shown beside the question editor

    Examples:
      | format     |
      | Markdown   |
      | txt        |
      | PDF        |
      | PowerPoint |
      | EPUB       |

  Scenario: Markdown and txt show line numbers
    When I open a Markdown file while writing questions in "CPU"
    Then the file is shown with line numbers

  Scenario: Files are not kept
    Given I opened "lecture-3.pdf" while writing questions in "CPU"
    When I finish writing questions
    Then "lecture-3.pdf" is not stored by the app

  Scenario: Several files in tabs
    When I open "lecture-3.pptx" and "lecture-4.pptx" while writing questions in "CPU"
    Then both files are open as tabs
    And I can switch between them while writing

  Scenario: Add a source to the bibliography
    When I add a source with the author "Patterson, D.", the year "2020", the title "Computer Organization and Design" and the publisher "Morgan Kaufmann"
    Then the bibliography lists it in APA format

  Scenario: Cite sources on a question
    Given the sources "Patterson 2020" and "Lecture 3" in the bibliography
    When I write a question in "CPU"
    And I cite "Patterson 2020" at page "42"
    And I cite "Lecture 3" at slide "12"
    And I save the question
    Then the question shows both citations with their locations

  Scenario: Question without a source
    When I write a question in "CPU" with no citation
    And I save the question
    Then the save is accepted

  Scenario: See which topics use a source
    Given a topic called "GPU"
    And questions in "CPU" and "GPU" that cite "Patterson 2020"
    When I look at "Patterson 2020" in the bibliography
    Then I see that "CPU" and "GPU" use it

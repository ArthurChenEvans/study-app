Feature: Export and import (R112 to R118)
  So that I can move everything to another computer, or share topics with a friend,
  in a file I can still read without the app

  Scenario: Moving everything to another computer
    Given months of quizzes, reviews, checklists and projects
    When I export everything with history and import it on another computer
    Then everything there is the same: the history, each quiz's results, the checklists, the projects and what's due
    And a quiz mistake I dismissed stays dismissed

  Scenario: Importing the same file twice
    When I import a file I've already imported
    Then nothing new is added and nothing is copied

  Scenario: Sharing some topics
    When I export the OSI model topic to share
    Then the file has its questions, cards, its checklist, its module and the sources it cites
    And no reviews, quizzes, projects or settings
    And my friend's copy starts every card as new

  Scenario: An updated version of a shared topic
    Given my friend imported my OSI model topic
    And I've since edited one of its questions
    When they import the new version
    Then the preview shows 1 question changed in both places, with mine newer
    And keeping theirs changes nothing
    And taking mine updates the question instead of adding a copy

  Scenario: Whichever changed last
    Given the same topic was edited here after it was edited in the file
    When I import with "Use whichever changed last"
    Then my edit stays

  Scenario: Both computers reviewed a card
    When I import a file where a card was reviewed after I last reviewed it here
    Then the review is added and the card takes that newer schedule
    But importing an older file never moves a schedule back

  Scenario: Keeping my version of a note keeps my cards
    Given the file's version of a cloze note has more blanks than mine
    When I keep mine
    Then I keep my cards, and taking the file's gives me its cards

  Scenario: Images go with the questions that show them
    When I share a topic with a question that shows a picture
    Then only that picture goes in the file
    And after importing, the question shows the same picture

  Scenario: An import makes a backup first
    When I import a file
    Then a backup marked "Before import" is made first

  Scenario Outline: Files that can't be trusted
    When I try to import <a file>
    Then I'm told "<why>"
    And nothing changes, and no backup is made

    Examples:
      | a file                                  | why                                                               |
      | that isn't a zip                        | it isn't a .studyapp file                                         |
      | made by a newer version of the app      | it was made by a newer version of the app, update this one first |
      | with a question in a topic it doesn't have | a question or card belongs to a topic that isn't in it         |
      | with two records sharing an ID          | two topics have the same ID, or one has none                      |
      | with an "image" that's really a script  | one of its images isn't a picture the app can show                |
      | with an image named ../../etc/passwd    | an image has a broken name                                        |
      | that unpacks to far more than its size  | it unpacks to more than the app will read                         |

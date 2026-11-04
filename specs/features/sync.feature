Feature: Sync through a folder (R119 to R128)
  So that my desktop, laptop and the browser version all have the same study,
  without paying for a server or making an account

  Background:
    Given Desktop and Laptop both sync through the same folder

  Scenario: Setting up the second computer
    Given Desktop has months of topics, cards, reviews and quizzes
    When I set up sync on an empty Laptop with the same folder
    Then Laptop has everything Desktop has, with the same history and the same things due
    And a backup marked "Before sync" was made on each first
    And the folder has only change files, never the database

  Scenario: Studying on both
    When I review cards and do quizzes on both computers before they sync
    Then after syncing both have every review and quiz
    And neither has anything to check

  Scenario: Editing different parts of a card
    When I change a card's front on Desktop and its back on Laptop
    Then after syncing both computers have the new front and the new back

  Scenario: Editing the same part on both
    When I change a card's back on Desktop and then on Laptop, before they sync
    Then after syncing both use Laptop's back
    And both list 1 thing to check, showing both versions
    And choosing Desktop's version puts it back on both computers

  Scenario: Deleting
    When I delete a topic on Desktop
    Then after syncing it and its questions and cards are gone on Laptop too

  Scenario: Deleting something the other computer changed
    When I delete a topic on Desktop
    And add a question to it on Laptop before they sync
    Then after syncing both still have the topic and the new question
    And both list it to check

  Scenario: Reviewing the same card on both
    When I review the same card on both computers before they sync
    Then both computers have both reviews
    And the card's schedule is the same on both, worked out from both reviews

  Scenario: Three computers in any order
    Given Desktop, Laptop and the browser version each make changes
    When they sync in any order, as often as they like
    Then all three end up with exactly the same study

  Scenario: Syncing again changes nothing
    When both computers have synced
    Then syncing again writes no new changes

  Scenario: Something arrives before what it belongs to
    Given the browser version made a topic and Laptop added a question to it
    When Desktop gets Laptop's changes before the browser version's
    Then the question waits
    And it appears once the topic arrives

  Scenario: The folder goes missing
    Given the sync folder is on a USB stick that's unplugged
    When I add 2 questions
    Then sync says it can't find the folder and 2 changes are waiting
    And when the stick is back they go out

  Scenario: A change file that's still being copied
    When Laptop's change file ends halfway through a line
    Then Desktop reads everything up to it and the rest next time

  Scenario: A computer on a newer version
    When Laptop has a newer version of the app
    Then Desktop skips its changes and says to update

  Scenario: A quiz in progress
    When a quiz is in progress
    Then the automatic sync waits until it's finished

  Scenario: Restoring a backup
    When I restore a backup with sync on
    Then sync is paused, and says why

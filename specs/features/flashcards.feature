@cards
Feature: Flashcards scheduled with FSRS
  As a student
  I want short cards for the things I keep forgetting, scheduled for just before I'd forget them
  So that reviews stay few and focused

  Background:
    Given a topic "CPU"
    And today is Tue 3 Nov 2026

  # R62, R63
  Scenario Outline: One note makes one or more cards
    When I save a <kind> note in "CPU"
    Then <count> cards are added, all new and due today

    Examples:
      | kind                                  | count |
      | Basic note                            | 1     |
      | Basic note with 2 more cues           | 3     |
      | Basic and reversed note               | 2     |
      | Cloze note with gaps c1, c2, c2 and c3 | 3     |

  Scenario: A cloze gap shows its hint and is filled in on the answer
    Given a cloze card "L3 takes about {{c1::40::tens of cycles}} cycles"
    When I review it
    Then the front reads "L3 takes about [tens of cycles] cycles"
    And the answer fills in "40"

  Scenario: Notes need content
    When I save a Basic note with no back
    Then I am told to write the back
    When I save a Cloze note with no gaps
    Then I am told to make at least one gap

  # R65, R66
  Scenario: Grading moves a card on
    Given a new card
    When I review it and press Good
    Then it is due on a later day
    And the buttons showed when it would come back for each answer

  Scenario: Again shows a card again in the same session
    Given 2 cards due
    When I press Again on the first
    Then it comes back after the second in the same session

  # R67
  Scenario: Undo the last answer
    When I press Good and then Undo
    Then the same card is shown again with its old schedule

  # R69
  Scenario: Daily limits
    Given 15 new cards and new cards a day is 10
    When I start a review
    Then 10 new cards are in the session

  # R70
  Scenario: A card that keeps slipping is flagged
    Given a card that has had Again 7 times and the threshold is 8
    When I press Again on it
    Then it is flagged as worth rewriting
    When I edit and save it
    Then the flag is cleared and it keeps its due date

  # R71
  Scenario: Archived cards are left out
    Given a due card
    When I archive it
    Then it isn't in the review
    When I bring it back
    Then it is due again

  # R72, R73
  Scenario: Suggestions from recalls and quizzes
    Given I missed "Memory-mapped I/O" in my last recall
    And I answered "What does the TLB cache?" wrong in a quiz
    When I open suggestions
    Then both are listed
    When I make a card from the quiz mistake
    Then the card starts with the question and its answer
    And the suggestion is gone
    When I dismiss the other
    Then no suggestions are left

  # R68, R76
  Scenario: The Cards screen and the done screen
    Given 3 cards due
    When I review all 3
    Then nothing is due today
    And the done screen shows 3 reviewed and the share remembered

  # R75
  Scenario: Fitting the scheduler needs history
    Given fewer than 50 reviews
    When I fit the scheduler
    Then I am told it needs more reviews first

  # R191
  Scenario: Review comes first
    When I open Cards
    Then I see what's due and can start a review, with no cards listed

  Scenario: Browsing every card without spoiling today
    When I open Browse all
    Then every card is listed
    And a card due today says "Hidden until you review it" instead of its answer

  Scenario: Narrowing the list
    When I search for "tag:exam is:new" and tick a topic in Filters
    Then only new exam cards in that topic are listed
    And folding the filters away keeps them applied

Feature: Toasts
  A calm note says what just happened, and deletes can be undone.

  # R155
  Scenario: Say what happened
    When I add a topic
    Then a toast says "Topic added"
    When I move 12 topics to Networks
    Then a toast says "12 topics moved to Networks" with Undo

  Scenario: Two at most
    When three toasts come quickly
    Then only the newest two show

  Scenario: Errors stay
    When saving fails
    Then an error toast is announced straight away and stays until I close it

  # R157
  Scenario: Undo a delete
    When I delete an empty topic
    Then it goes at once with a toast and Undo
    When I press Undo
    Then it comes back and nothing was deleted

  Scenario: Big deletes still ask
    When I delete a topic with questions
    Then I'm asked first
    And after confirming it goes with Undo for six seconds

  Scenario: Closing the app finishes waiting deletes
    Given I deleted a topic a moment ago
    When I close the app
    Then it's deleted for real

Feature: Navigation on every screen size
  The navigation holds places you go, grouped, and changes shape to fit the screen.

  # R158
  Scenario: A tidy sidebar
    Given the window is 1100px or wider
    Then the links come in Today, Study and Library groups
    And Search is a box at the top
    And theme, sound and celebrations are on the Settings screen, not in the sidebar

  # R159
  Scenario: A tablet gets a rail
    Given the window is 800px wide
    Then the navigation is a rail of icons with short labels
    When I press More
    Then Bibliography, History, New quiz and Your data are there

  # R160
  Scenario: A phone gets a dock
    Given the window is 390px wide
    Then the dock shows Today, Calendar, More, Topics and Cards, More in the middle
    When I press More
    Then a sheet lists Recall, Projects, Bibliography, History, Search, New quiz, Your data and Settings

  # R161
  Scenario: Nothing scrolls sideways
    Given the window is 390px wide
    When I open any screen
    Then the page is no wider than the window

  Scenario: Picking a row on a phone
    Given the window is 390px wide
    When I pick a topic
    Then its details come up as a sheet I can close

  # R162
  Scenario: Topics scroll down on a phone
    Given the window is 390px wide
    When I open Topics
    Then each topic is a card with its questions, quiz step and next quiz labelled
    And the page scrolls down, never sideways
    And the column names are chips I can sort by

  # R163
  Scenario: A week at a time
    When I pick Week on the calendar
    Then I see Monday to Sunday as a list with everything on each day in full
    When I press Next week
    Then the next seven days show
    And the app remembers Week next time

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

@appearance
Feature: Light and dark theme
  As a student
  I want to choose between a dark and a light theme
  So that the app is comfortable to use at any time of day

  Scenario: Dark by default
    Given I have never chosen a theme
    When I open the app
    Then it is shown in the dark theme

  Scenario: Switch to light
    Given the app is in the dark theme
    When I switch to the light theme
    Then the app is shown in the light theme

  Scenario: Choice is remembered
    Given I switched to the light theme
    When I close and reopen the app
    Then it is shown in the light theme

  Scenario: Colour-blind safe colours (R129)
    Given right answers are green and wrong ones red
    When I turn on "Colour-blind safe" at the bottom of the sidebar
    Then right answers are blue and wrong ones orange, everywhere
    And every answer still shows an icon and a word, and guessed is still hatched
    And the choice is remembered next time

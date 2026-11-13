Feature: Updates
  The app updates itself from signed releases, never from half-finished work.

  # R165
  Scenario Outline: Only a newer release is an update
    Given version 0.1.7 is installed
    When the newest release is <release>
    Then <outcome>

    Examples:
      | release | outcome                       |
      | 0.1.8   | it's offered as an update     |
      | 0.1.7   | I'm told I have the newest    |
      | 0.1.6   | I'm told I have the newest    |

  # R166
  Scenario: Auto update downloads in the background
    Given Auto update is on
    When I open the app and a newer release is out
    Then it downloads without asking
    And a toast says "Version 0.1.8 is ready" with Install now
    And it says my computer will ask for my password

  Scenario: Installing on close
    Given an update has downloaded
    When I close the app
    Then it says "Installing the update"
    And the system asks for my password
    And the app closes when it's done

  Scenario: Auto update off
    Given Auto update is off
    When I open or close the app
    Then it doesn't check or install anything

  # R167
  Scenario: Checking by hand
    When I press Check for updates in Settings
    Then I see the new version, its notes and Update to 0.1.8
    When I press it
    Then it downloads, backs up, asks for my password and restarts

  # R168
  Scenario: Backed up first
    When an update installs
    Then a "before update" backup was made first
    And my study data is still there afterwards

  Scenario: A tampered package is refused
    When the downloaded package doesn't match its signature
    Then it isn't installed
    And Settings says "The download didn't match its signature, so it wasn't installed."

  Scenario: Offline
    When I'm offline
    Then opening and closing the app says nothing
    And Check for updates says it couldn't reach GitHub

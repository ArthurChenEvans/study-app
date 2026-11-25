Feature: Accounts are optional on the desktop (R221)

  Rule: no account is needed on the desktop, and where to get one is easy to find but never pushed

  Scenario: First launch
    Given the desktop app has never been opened
    When it opens
    Then it opens straight onto the work
    And nothing asks to sign in or make an account

  Scenario: Finding where to sync
    Given nothing syncs yet, by folder or account
    Then the sidebar shows a quiet "Sync across devices" link to Settings
    And there are no banners, nag dialogs or locked features

  Scenario: The link goes once something syncs
    Given the computer is signed in, or syncs with a folder
    Then the sidebar doesn't show "Sync across devices"

  Scenario: The phone still needs an account
    Given the phone version
    Then it opens on signing in (R211)

  Rule: signing in with data on both sides asks which to keep, and never merges

  Scenario: Data here and in the account
    Given this computer has 4 topics and 120 questions
    And the account already syncs with "Laptop"
    When I sign in
    Then "Keep which data?" asks me to pick this computer or the account
    And it says what each side has
    And nothing syncs until I pick

  Scenario: Keeping this computer's data
    Given "Keep which data?" is open
    When I pick "This computer" and press "Replace the account's data"
    Then a backup is made here
    And the account's data is cleared and replaced with this computer's

  Scenario: Keeping the account's data
    Given "Keep which data?" is open
    When I pick "The account" and press "Replace this computer's data"
    Then a backup is made here
    And this computer is emptied and takes in the account's data

  Scenario: Backing out
    Given "Keep which data?" is open
    When I press "Sign out instead"
    Then I'm signed out and both sides are left as they were

  Scenario: Closing the app before picking
    Given "Keep which data?" is open
    When I close the app and open it again
    Then it still asks, and still nothing has synced

  Scenario: Only one side has data
    Given only this computer, or only the account, has data
    When I sign in
    Then nothing asks, and that data is used

  Scenario: Signing back in
    Given this computer synced with the account before and was signed out
    When I sign in again
    Then nothing asks, and it carries on syncing

  Scenario: The account can't be checked
    Given this computer has data and Supabase doesn't answer
    When I sign in
    Then it asks which to keep rather than merging

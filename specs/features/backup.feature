Feature: Backups (R101 to R108)
  So that months of study history can't be lost to one broken file
  I want the app to back itself up, and to put a backup back when I need it

  Scenario: The first backup of the day is made automatically
    Given the database has a topic
    And there's no backup from today
    When the app starts or I move between screens
    Then a backup marked "Daily" is made
    And no second daily backup is made that day

  Scenario: An empty database isn't backed up automatically
    Given the database has nothing in it
    When the app starts
    Then no backup is made

  Scenario: Backing up by hand
    When I press "Back up now" on Your data
    Then a backup marked "By hand" is listed first with its size
    And the status reads "Backed up today at" the time

  Scenario: Old backups are thinned out
    Given backups from every day for the last 400 days
    When a new backup is made
    Then the newest backup of each of the last 7 days is kept
    And the newest of each of the last 4 weeks
    And the newest of each of the last 12 months
    And the rest are deleted

  Scenario: Restoring shows what will be undone
    Given a backup from Mon 2 Nov
    And I've reviewed 183 cards and done 3 quizzes since
    When I choose Restore on that backup
    Then I'm told "183 card reviews" and "3 quizzes" will be undone
    And Cancel has the focus

  Scenario: Restoring keeps a way back
    When I restore the backup from Mon 2 Nov
    Then a backup marked "Before restore" is made first
    And the data is as it was on Mon 2 Nov

  Scenario: Restoring a backup from an older version
    Given a backup made before the last database upgrade
    When I restore it
    Then it's upgraded and opens normally

  Scenario: An app update backs up first
    Given a database from an older version of the app
    When the new version opens it
    Then a backup marked "Before update" is made before anything changes

  Scenario: Copying backups to a second folder
    Given I've set backups to also copy to a folder on a USB stick
    When a backup is made
    Then the same file is in that folder, thinned out the same way
    But if the folder can't be reached
    Then the backup is still made here, and Your data says it couldn't be copied

  Scenario: Restoring doesn't change backup settings
    Given I keep backups for 14 days
    When I restore a backup from before I changed that
    Then I still keep backups for 14 days

Feature: IDs, change stamps and deletion markers (R109 to R111)
  So that export, import and sync can tell which records are the same on two computers,
  which is newer, and which were deleted

  Scenario: Every record gets an ID
    When I make a module, a topic, two questions and a project
    Then each has its own ID, different from every other
    And each is stamped as changed on this computer

  Scenario: Editing moves the stamp, including a record's parts
    Given a topic with a question
    When I move the topic to a module, or change its tags
    Then the topic's stamp moves on and its ID stays the same
    When I change one of the question's options
    Then the question's stamp moves on

  Scenario: Reviewing a card isn't an edit
    When I review a card
    Then its stamp stays where it was
    But when I archive it, the stamp moves on

  Scenario: Deleting leaves a marker, for everything deleted along with it
    When I delete a topic with a question and a card in it
    Then there's a deletion marker for the topic, the question, the note and the card
    And none for the topics I kept

  Scenario: Settings are stamped too
    When I change the quiz schedule
    Then the setting is stamped
    And resetting it leaves a marker

  Scenario: Records from before the update get IDs too
    Given a database from before IDs
    When the app opens it
    Then every record has an ID and a stamp
    And this computer keeps the same ID every time the database is opened

  Scenario: Restoring a backup from before IDs
    When I restore a backup made before IDs
    Then its records get IDs
    And their stamps come after everything already stamped here

Feature: Importing and exporting other files (R171 to R178)
  So that decks and questions I made elsewhere come in, and mine go out to other apps

  Scenario: Anki decks
    Given Networks.apkg has decks Networks::DNS, Networks::TLS and Misc
    When I import it
    Then each deck is shown with where it goes, a new topic in the Networks module by default
    And Image Occlusion notes are flagged "Check these"
    When I put TLS into my TLS handshakes topic and skip Misc
    Then the button says how many notes come in, leaving out the ones already here
    And afterwards a toast offers Undo

  Scenario: Keeping Anki's progress
    When I import a deck and keep Anki's progress
    Then a card due in 3 days in Anki is due in 3 days here
    And cards still being learned are due today

  Scenario: Exporting for Anki
    When I export everything as an Anki deck
    Then Anki opens it with every card, cloze, image and formula

  Scenario: Spreadsheets
    When I import cpu-questions.csv
    Then its columns are matched by their names
    When I say column 7 is tags
    Then the preview updates
    And a row whose answer is E with 3 options is listed with why, and left out

  Scenario: Exporting for Moodle
    When I export my questions as Moodle GIFT
    Then my history isn't asked about

  Scenario: Undoing the last import
    Given I imported Networks.apkg today at 14:20
    Then Your data shows it as the last import
    When I undo it and confirm
    Then everything is back to how it was at 14:20

  Scenario: A file it can't read
    When I import lecture-notes.zip
    Then it says it can't import that kind of file, nothing was added, and offers another

  Scenario: Importing a big deck
    When an import has thousands of notes
    Then it shows how far it's got
    And stopping it adds nothing

  Scenario: Dropping a file on the window
    When I drag Networks.apkg over the window
    Then it says "Drop to import"
    But dragging an image over a card field adds the image there instead

@topics
Feature: Organise topics into modules and tags
  As a student
  I want to group my questions into topics, modules and tags
  So that I can find and quiz on the right material

  Scenario: Create a topic
    When I create a topic called "CPU"
    Then "CPU" appears in my topics
    And it has no questions

  Scenario: Put a topic in a module
    Given a module called "Computer Architecture"
    When I create a topic called "CPU" in the module "Computer Architecture"
    Then "CPU" is listed under "Computer Architecture"

  Scenario: Topic outside any module
    When I create a topic called "Misc" with no module
    Then "Misc" is listed outside any module

  @edge
  Scenario: Move a topic to another module
    Given a topic "CPU" in the module "Computer Architecture"
    And a module called "Hardware"
    When I move "CPU" to the module "Hardware"
    Then "CPU" is listed under "Hardware"
    And "CPU" is no longer listed under "Computer Architecture"

  Scenario: Tag a topic
    Given a topic called "GPU"
    When I tag "GPU" with "hardware" and "exam-1"
    Then "GPU" has the tags "hardware" and "exam-1"

  @edge
  Scenario: Delete a topic asks for confirmation
    Given a topic "CPU" with 5 questions
    When I delete "CPU"
    Then I am asked to confirm
    And "CPU" is still in my topics until I confirm

  @edge
  Scenario: Delete a topic that is in history
    Given a topic "CPU" with 5 questions
    And a finished attempt that included questions from "CPU"
    When I delete "CPU" and confirm
    Then "CPU" no longer appears in my topics
    And the attempt in history still shows the "CPU" questions as they were

  Scenario: Renaming a module or a topic (R185)
    Given the module "Computer Architecture" with the topic "CPU"
    When I pick the module in the rail and choose Module, then Rename module…
    And I call it "Architecture"
    Then the rail and its recall checklist both say "Architecture"
    When I pick CPU and choose Rename
    Then I can give it a new name, and an empty name says why it can't be saved

  # R189
  Scenario: Making a quiz
    When I make a new quiz called "GPU" in Computer Architecture
    Then the question editor opens for GPU

  Scenario: Deleting a quiz keeps the topic
    When I delete the CPU quiz
    Then its questions go, and CPU stays with its cards and checklists

  Scenario: Looking after topics and modules
    When I pick TLS on Topics & Modules
    Then I see how many questions, cards and checklists it has
    And I can rename it, move it, tag it or delete it

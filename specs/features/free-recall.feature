@recall
Feature: Free recall with a checklist
  As a student
  I want to dump everything I remember about a module or topic and tick it off against a checklist
  So that the app schedules my next recall from how well I did

  Background:
    Given a module "Computer Architecture" with topics "CPU" and "Memory"
    And today is Tue 3 Nov 2026

  # R52
  Scenario: Make a checklist about a module or a topic
    When I make a checklist about the module "Computer Architecture"
    And I make a checklist about the topic "CPU"
    Then both checklists are listed and due today

  # R53
  Scenario: Write, reorder and remove points
    Given a checklist about "Computer Architecture"
    When I add the points "Pipeline stages", "Cache levels" and "Amdahl's law"
    And I move "Amdahl's law" to the top
    And I remove "Cache levels"
    Then the points are "Amdahl's law" then "Pipeline stages"

  # R54
  Scenario: Add and remove points from questions
    Given an MCQ in "CPU" asking "Which stage reads registers?" with the correct answer "Decode"
    And a checklist about "Computer Architecture"
    When I add that question to the checklist
    Then the checklist has the point "Which stage reads registers? → Decode"
    When I untick that question
    Then the point is removed and the question is unchanged

  Scenario: A checklist needs a point before it can start
    Given a checklist about "CPU" with no points
    When I try to start a recall
    Then I am told to add at least one point

  # R55
  Scenario Outline: Dump in the app or outside it
    Given a checklist about "CPU" with 10 points
    When I do the dump <where>
    And I finish the check
    Then the check is saved <saved>

    Examples:
      | where           | saved                  |
      | typed in        | with what I typed      |
      | outside the app | without a typed dump   |

  # R56, R57
  Scenario Outline: The step moves up at the pass mark and back under it
    Given a checklist about "CPU" with 10 points on step "<from>"
    When I tick <ticked> points on Tue 3 Nov
    Then the score is <score>%
    And the checklist is on step "<to>"
    And the next check is on <next>

    Examples:
      | from          | ticked | score | to            | next      |
      | Month         | 7      | 70    | Monthly 1 of 6 | Thu 3 Dec |
      | Month         | 6      | 60    | Fortnight     | Tue 17 Nov |
      | Monthly 6 of 6 | 9     | 90    | Seasonal 1 of 4 | Wed 3 Feb |
      | Yearly        | 8      | 80    | Yearly        | Wed 3 Nov 2027 |
      | Day           | 2      | 20    | Day           | Wed 4 Nov |

  Scenario: The first check puts a new checklist on Day
    Given a new checklist about "CPU" with 10 points
    When I tick 3 points
    Then the checklist is on step "Day"
    And the next check is tomorrow

  # R57 (answer B): late checks count from the day they were done
  Scenario: A late check counts from the day it was done
    Given a checklist on step "Week" that was due on Sun 1 Nov
    When I pass the check on Tue 3 Nov
    Then the next check is on Tue 17 Nov

  Scenario: The result shows what was missed and how often
    Given "Memory-mapped I/O" was missed in 3 of the last 4 checks
    When I miss it again
    Then the result lists it as missed 4 of 5 checks

  # R58
  Scenario: Due checklists come first
    Given checklists due today, on Thu 5 Nov and on 14 Jan
    When I open Recall
    Then the one due today is listed under "Due today"
    And the others are listed by next date
    And the sidebar shows 1 due

  # R59
  Scenario: History of a checklist
    Given a checklist with 5 checks
    When I open its history
    Then I see the score of each check, its step change and whether it had a typed dump
    And the points missed most often

  # R60
  Scenario: Edit and reset the schedule
    When I change the pass mark to 80% and Monthly to repeat 3 times
    Then the next checks use the new schedule
    When I reset the schedule
    Then it is back to the default ladder with a 70% pass mark

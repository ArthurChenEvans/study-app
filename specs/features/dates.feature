Feature: Dates on the calendar

  # R130
  Scenario: Add a date
    Given it's Tue 3 Nov
    When I add "Study app write-up" on Fri 13 Nov as a Deadline, covering nothing
    Then the calendar shows it on 13 Nov with a flag
    And "Coming up" says "Fri 13 Nov, in 10 days"

  # R130
  Scenario: Edit and delete a date
    Given the date "Study app write-up" on Fri 13 Nov
    When I move it to Mon 16 Nov
    Then it shows on 16 Nov
    When I delete it
    Then it's gone from the calendar

  # R131
  Scenario: A date covering a module covers topics added later
    Given "Networks exam" covers the Networks module
    When I add the topic "Routing" to Networks
    Then the exam covers Routing too

  # R131
  Scenario: A deleted topic drops out
    Given "Architecture mock" covers CPU and Caches
    When I delete Caches
    Then the mock covers only CPU

  # R132
  Scenario: Tightening is on by default for exams
    When I add a date with the kind Exam covering a module
    Then "Tighten reviews before this" is ticked
    When the date covers nothing
    Then it can't be ticked

  # R132, R133
  Scenario Outline: A quiz is pulled in to half the days that were left
    Given TCP/IP was quizzed on <last> and its next quiz is <normal>
    And "Networks exam" on Mon 23 Nov covers TCP/IP and tightens reviews
    Then TCP/IP's next quiz is <tightened>

    Examples:
      | last       | normal     | tightened  |
      | Tue 3 Nov  | Sat 28 Nov | Fri 13 Nov |
      | Tue 3 Nov  | Thu 5 Nov  | Thu 5 Nov  |
      | Sun 22 Nov | Sun 6 Dec  | Sun 6 Dec  |

  # R133
  Scenario: Something done long ago comes up straight away
    Given TCP/IP was last quizzed on Mon 5 Oct, next quiz Sat 5 Dec
    And it's Tue 3 Nov and "Networks exam" on Mon 23 Nov tightens TCP/IP
    Then TCP/IP is due today

  # R132
  Scenario: Cards and drills are tightened, checklists aren't
    Given a Networks card and a Networks drill due after the exam
    And a Networks recall checklist due after the exam
    When "Networks exam" tightens the Networks module
    Then the card and the drill come in before the exam
    And the checklist stays where it was

  # R132
  Scenario: The tighter date wins
    Given "Networks mock" on Fri 13 Nov and "Networks exam" on Mon 23 Nov both tighten TCP/IP
    Then TCP/IP's next quiz is worked out from the mock

  # R133
  Scenario: After the date things go back to normal
    Given "Networks exam" on Mon 23 Nov tightened TCP/IP to Fri 13 Nov
    And I didn't quiz it
    When it's Tue 24 Nov
    Then TCP/IP's next quiz is its normal Sat 28 Nov

  # R133
  Scenario: Deleting the date puts everything back
    Given "Networks exam" tightened TCP/IP to Fri 13 Nov
    When I delete the exam
    Then TCP/IP's next quiz is Sat 28 Nov again

  # R134
  Scenario: Dates on the month view
    Given "Networks exam" on Mon 23 Nov and "Study app write-up" on Fri 13 Nov
    When I open the calendar on Tue 3 Nov
    Then 23 Nov has an exam chip and 13 Nov a flag chip
    And "Coming up" lists both with the days left
    And the days before the exam count the tightened quizzes

  # R135
  Scenario: The exam's day
    Given "Networks exam" covers OSI model, TCP/IP and Subnetting, which was never quizzed
    When I open Mon 23 Nov
    Then the exam's panel says it's in 20 days and covers the Networks module
    And it lists TCP/IP's next quiz Fri 13 Nov, pulled in from Sat 28 Nov
    And Subnetting with a Quiz button
    And how many Networks cards are due before it and how many were pulled in

  # R135
  Scenario: The week before
    When I open Mon 16 Nov
    Then it says "Networks exam in 7 days"
    And a quiz due that day says it's tightened for the Networks exam

  # R136
  Scenario: Preview before saving
    Given TCP/IP's next quiz is Sat 28 Nov
    When I start adding "Networks exam" on Mon 23 Nov covering Networks
    Then it says "TCP/IP quiz moves from Sat 28 Nov to Fri 13 Nov"
    And "Subnetting hasn't been quizzed yet"
    And how many cards come in before it

  # R137
  Scenario: Tightened next quizzes are labelled
    Given TCP/IP's next quiz is tightened for the Networks exam
    Then Topics shows "Tightened for Networks exam" under its next quiz
    When I finish a TCP/IP quiz
    Then results show the tightened next quiz and what it would have been

  # R137
  Scenario: Dates sync
    Given sync is on between Desktop and Laptop
    When I add "Networks exam" on Desktop
    Then it shows on Laptop after a sync

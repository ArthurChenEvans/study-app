Feature: Quiz scheduling and the calendar

  # R96
  Scenario Outline: A topic's quiz schedule moves with its score
    Given "CPU" is on the Week step of the quiz schedule
    When I finish a quiz where CPU scored <score>
    Then CPU is on the <step> step, next quiz <next>

    Examples:
      | score | step      | next             |
      | 86%   | Fortnight | in two weeks     |
      | 50%   | Day       | the next day     |

  # R96
  Scenario: Retries and quits don't move the schedule
    Given "CPU" is on the Week step
    When I finish a retry of a CPU quiz, or quit one halfway
    Then CPU stays on the Week step

  # R96
  Scenario: Overruling redoes the move
    Given a quiz moved CPU back to Day
    When I overrule a CPU answer on the results page so CPU passes
    Then CPU moves up from where it was instead

  # R98, R99
  Scenario: See what's when
    Given cards, a checklist, a quiz and a drill due on different days
    When I open the calendar
    Then each day shows what's due on it with counts
    And overdue things show on today
    When I open a past day
    Then it shows what I did and the struggles I logged

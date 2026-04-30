Feature: Traffic Tab Filtering

  Scenario: User sets method filter to GET
    Given the traffic tab is active
    And the request list contains "GET" and "POST" requests
    When the user presses "m" to enter method filter mode
    Then the filter bar shows "[m]ethod: *"
    When the user types "GET"
    And waits for the filter to apply
    Then only "GET" requests appear in the list
    And the filter bar shows "[GET]"

  Scenario: User clears filter with Escape
    Given the traffic tab has a method filter "[GET]" active
    When the user presses Esc
    Then all requests are shown again
    And the filter bar shows "[*]"

  Scenario: User starts proxy with 'r' key
    Given the traffic tab is active
    When the user presses "r"
    Then the proxy starts
    And the status shows "proxy running"

  Scenario: User quits with 'q'
    Given the TUI is running
    When the user presses "q"
    Then the application exits successfully

  Scenario: User navigates tabs with Tab key
    Given the traffic tab is active
    When the user presses Tab
    Then the rules tab becomes active
    When the user presses Tab 8 more times
    Then the traffic tab is active again (wrapped)

  Scenario: User searches with forward slash
    Given the traffic tab is active
    And there are requests in the list
    When the user presses "/"
    Then the search input is focused
    And the filter bar shows "/regex/"
    When the user types "api"
    Then only requests matching "api" appear
    And the filter bar shows "/api/"

  Scenario: User navigates request list with j/k
    Given the traffic tab is active
    And there are multiple requests in the list
    When the user presses "j" (down)
    Then the selection moves down one row
    When the user presses "k" (up)
    Then the selection moves up one row

  Scenario: User toggles pf with 'p'
    Given the traffic tab is active
    When the user presses "p"
    Then pf is toggled
    And the controls bar shows the new pf state

  Scenario: User views request detail with Enter
    Given the traffic tab is active
    And there are requests in the list
    When the user presses Enter on a selected request
    Then the detail panel shows the request headers
    When the user presses "2"
    Then the detail panel shows the request body
    When the user presses "3"
    Then the detail panel shows WebSocket frames (if available)
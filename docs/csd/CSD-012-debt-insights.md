---
id: CSD-012
title: Debt insights
status: approved
owner: Thijs Bardoel
version: 0.3
last_updated: 29/10/2026
related: [CSD-010, CSD-011]
---

# CSD-012: Debt insights

## 1. Intent
The Debts page lists debts, statements and a table of yearly totals, but a table does not show how the debts shrink and which debt is shrinking. This capability adds an insights layer to the Debts page: a chart with one bar per calendar year, stacked from one segment per debt, with a marker at the total debt of the year and the amount repaid in the tooltip. It is for the owner looking back over their own debts. It follows the Bank account insights of CSD-010, with the debts as segments and without negative values, because a debt balance is never negative. It supersedes CSD-011 NG-2 for this view only to the extent that the chart shows the balance per debt, and not any percentage or forecast.

## 2. Scope
### 2.1 In scope
- An "Insights" tab on the Debts page, next to the existing content ("Overview").
- A yearly stacked bar chart over all debts with one segment per debt, a total-debt marker, a legend of the debts, a tooltip and the highlight of one debt.
- Its empty state.

### 2.2 Non-goals
- NG-1: A monthly view, or any granularity other than a calendar year.
- NG-2: A filter, a selection or a hiding of debts.
- NG-3: A percentage of the debt repaid, a payoff forecast, and any figure other than the balances, the total debt, the amount repaid.
- NG-4: Click-through, zooming, or exporting the chart.
- NG-5: Charts on the Dashboard or on the Reports page.
- NG-6: A chart library or any other new dependency.
- NG-7: Estimating balances for years without a statement (CSD-011 NG-11), and carrying a balance forward into a year in which a debt has no statement.
- NG-8: Letting the user choose the colour of a debt.

## 3. Definitions
| Term | Meaning |
|---|---|
| **Year** | A calendar year. A statement counts in the year of its own period only, as in the yearly totals (CSD-011 REQ-19). |
| **Current year** | The calendar year of today, in local time. Its figures run up to the latest statement so far, and are therefore partial. |
| **Segment** | The part of a year's bar that belongs to one debt: the balance of that debt's latest statement in that year. A debt without a statement in that year has no segment in it. |
| **Total debt (of a year)** | The sum of the segments of a year, which is the closing balance of the yearly totals of CSD-011 REQ-19. |
| **Repaid** | As in CSD-011 section 3. |
| **Colour slot** | The number from 0 to 7 stored with a debt (CSD-011 REQ-3), which decides its colour. |

## 4. Requirements

### 4.0 Overview flow
```mermaid
sequenceDiagram
    actor U as User
    participant UI as Frontend (ui/)
    participant CMD as commands/
    participant DOM as domain/
    U->>UI: opens the Insights tab
    UI->>CMD: get_debt_insights()
    CMD->>DOM: insights(today)
    DOM-->>CMD: debts and one figure set per year, with segments
    CMD-->>UI: figures as decimal strings
    UI->>U: stacked bars per debt, total markers, legend
```

**REQ-1: Tabs on the Debts page** *(ubiquitous)*
The Debts page shall offer the tabs "Overview" and "Insights" while at least one debt exists, with Overview, which holds the content of CSD-011, shown first.

```gherkin
@AC-1.1
Scenario: Overview is shown first
  Given a project with one debt
  When the user opens the Debts page
  Then the Overview tab is selected
  And it shows the Debts, Yearly totals and Statements sections

@AC-1.2
Scenario: Switching to Insights
  Given the Debts page shows the Overview tab
  When the user selects the Insights tab
  Then the Overview sections are hidden
  And the insights chart is shown

@AC-1.3
Scenario: No debts yet
  Given a project without debts
  When the user opens the Debts page
  Then the prompt to add the first debt is shown
  And there are no tabs
```

**REQ-2: Year series** *(ubiquitous)*
The app shall return one set of figures for every calendar year from the year of the earliest statement up to and including the current year, oldest year first, holding the total debt, the amount repaid, the number of statements (used to detect an empty year, not displayed) and, per debt with a statement in that year, its segment.

```gherkin
@AC-2.1
Scenario: Series runs from the first statement to the current year
  Given today is 26/10/2026
  And the earliest statement is for 2024-03
  And there are statements in 2025 and 2026
  When the insights are requested
  Then the series holds the years 2024, 2025 and 2026 in that order

@AC-2.2
Scenario: A single year
  Given today is 26/10/2026
  And the only statement is for 2026-09
  When the insights are requested
  Then the series holds only the year 2026

@AC-2.3
Scenario: No statements
  Given today is 26/10/2026
  And a debt without statements
  When the insights are requested
  Then the series is empty
```

**REQ-3: Years without statements** *(unwanted behaviour)*
If a year in the series has no statement, then the app shall return that year with total debt 0.00, repaid none, 0 statements and no segments.

```gherkin
@AC-3.1
Scenario: Gap year
  Given today is 26/10/2026
  And there are statements in 2022 and 2024 but none in 2023
  When the insights are requested
  Then the year 2023 has total debt 0.00, repaid none, 0 statements and no segments
  And the series holds the years 2022, 2023, 2024, 2025 and 2026
```

**REQ-4: Current year is partial** *(state-driven)*
While a year is the current year, the app shall mark it as partial.

```gherkin
@AC-4.1
Scenario: Current year is marked
  Given today is 26/10/2026
  And there are statements in 2025 and 2026
  When the insights are requested
  Then the year 2026 is marked partial
  And the year 2025 is not

@AC-4.2
Scenario: Current year without statements is still in the series and marked
  Given today is 26/10/2026
  And the only statement is for 2025-12
  When the insights are requested
  Then the series holds 2025 and 2026
  And 2026 is marked partial and has 0 statements
```

**REQ-5: Segments are the latest balances per debt** *(ubiquitous)*
The app shall give each year one segment per debt that has a statement in that year, equal to the balance of that debt's latest statement in that year, ordered by the creation of the debts, and the segments shall add up to the total debt, which equals the closing balance of the yearly totals of CSD-011 REQ-19.

```gherkin
@AC-5.1
Scenario: Segments of a year
  Given today is 26/10/2026
  And debt "Example Mortgage" with statements 2026-03 (balance 151000.00) and 2026-09 (balance 150000.00)
  And debt "Example Student Loan" with a statement 2026-06 (balance 8000.00)
  When the insights are requested
  Then 2026 has the segments "Example Mortgage" 150000.00 and "Example Student Loan" 8000.00
  And 2026 has total debt 158000.00

@AC-5.2
Scenario: A debt without a statement in the year has no segment
  Given today is 26/10/2026
  And a monthly debt "Example Mortgage" with statements 2025-12 (balance 10000.00) and 2026-03 (balance 9500.00)
  And a yearly debt "Example Student Loan" with a statement 2025 (balance 6000.00)
  When the insights are requested
  Then 2025 has the segments "Example Mortgage" 10000.00 and "Example Student Loan" 6000.00
  And 2026 has only the segment "Example Mortgage" 9500.00

@AC-5.3
Scenario: A repaid debt has no visible segment
  Given today is 26/10/2026
  And debt "Example Mortgage" with a statement 2026-09 (balance 150000.00)
  And debt "Example Car Loan" with a statement 2026-09 (balance 0.00)
  When the insights are requested
  Then 2026 has the segments 150000.00 and 0.00
  And 2026 has total debt 150000.00

@AC-5.4
Scenario: Segments are ordered by creation, not by name
  Given today is 26/10/2026
  And debts "Example Student Loan" and then "Example Mortgage", both with a statement 2026-09
  When the insights are requested
  Then the segments of 2026 are listed "Example Student Loan" first and "Example Mortgage" second

@AC-5.5
Scenario: The insights equal the yearly totals
  Given today is 26/10/2026
  And a monthly debt with statements 2025-11 (balance 10000.00), 2025-12 (balance 9800.00) and 2026-03 (balance 9000.00)
  And a yearly debt with statements 2024 (balance 7000.00) and 2025 (balance 6000.00)
  When the insights and the yearly totals are requested
  Then for every year the total debt and the amount repaid of the insights equal those of the yearly totals
```

**REQ-6: Chart** *(ubiquitous)*
The Insights tab shall draw one bar per year in the order received, stacked upwards from the zero line from one segment per debt in the order received, on a y-axis that starts at 0 and ends at a rounded-up value of at least the highest total debt, with gridlines, compact axis labels (such as "10k"), a gap year as a labelled slot without a bar, and no currency.

```gherkin
@AC-6.1
Scenario: Stacked bars per year
  Given the series holds the years 2025 and 2026
  When the user opens the Insights tab
  Then two bars are drawn, 2025 on the left and 2026 on the right
  And each bar has one segment per debt with a statement in that year

@AC-6.2
Scenario: Axis covers the highest level
  Given the highest total debt of any year is 158000.00
  When the user opens the Insights tab
  Then the top of the y-axis is at least 158000.00
  And the axis starts at 0

@AC-6.3
Scenario: A repaid debt adds no height
  Given 2026 has the segments 150000.00 and 0.00
  When the user opens the Insights tab
  Then the 2026 bar is 150000.00 high
  And the second debt is still named in the legend and the tooltip

@AC-6.4
Scenario: Gap year is an empty slot
  Given the year 2023 has no statements
  When the user opens the Insights tab
  Then 2023 has a labelled slot without a bar

@AC-6.5
Scenario: Current year is marked
  Given the year 2026 is partial
  When the user opens the Insights tab
  Then the 2026 bar has a lighter fill
  And its label reads "2026 YTD"
```

**REQ-7: Total-debt marker** *(ubiquitous)*
The Insights tab shall draw for each year with statements a horizontal marker line of 2 px with round ends in the muted text colour, wider than the bar, at the total debt, with its compact value next to it (above the line), not connected to the markers of other years, and dashed for the current year.

```gherkin
@AC-7.1
Scenario: Marker at the total debt
  Given 2025 has the segments 2000.00 and 8000.00 and total debt 10000.00
  When the user opens the Insights tab
  Then a marker line is drawn across the 2025 bar at 10000.00
  And its label reads "10k"

@AC-7.2
Scenario: Marker sits at the top of the stack
  Given 2025 has the segments 2000.00 and 8000.00
  When the user opens the Insights tab
  Then the 2025 marker sits at the top of the stack

@AC-7.3
Scenario: Markers are not connected and the current year is dashed
  Given the series holds 2024, a gap year 2025 and the partial year 2026
  When the user opens the Insights tab
  Then no line joins the markers
  And the marker of 2026 is dashed
  And 2025 has no marker
```

**REQ-8: A colour per debt** *(ubiquitous)*
The Insights tab shall draw every segment of a debt in the colour of the debt's colour slot, taken from the palette of eight colours with a variant for light mode and one for dark mode that CSD-010 REQ-8 defines, so that a debt has the same colour in every year, in the legend and in the tooltip, and keeps it when other debts are deleted.

```gherkin
@AC-8.1
Scenario: A debt has one colour everywhere
  Given debt "Example Student Loan" has colour slot 1 and statements in 2025 and 2026
  When the user opens the Insights tab
  Then its segments in 2025 and 2026, its legend swatch and its tooltip swatch have the colour of slot 1

@AC-8.2
Scenario: Colours survive a deletion
  Given debts with colour slots 0, 1 and 2 and statements
  When the user deletes the debt with slot 0 on the Overview tab and opens the Insights tab
  Then the other debts are still drawn in the colours of slots 1 and 2

@AC-8.3
Scenario: The palette in both modes
  Given the Insights tab shows the chart
  When the app is in light mode and then in dark mode
  Then the eight colours are distinct from each other in each mode
  And each has at most 60% HSL saturation

@AC-8.4
Scenario: More than eight debts
  Given nine debts with statements
  When the user opens the Insights tab
  Then the first and the ninth debt share a colour
  And each is named in the legend and in the tooltip
```

**REQ-9: Legend of debts** *(ubiquitous)*
The Insights tab shall show a legend with a colour swatch and the name of every debt that has at least one statement, in the order the debts were created, wrapping onto more lines when it does not fit, with no limit on the number of debts, and shall cut a name that does not fit on one legend item with an ellipsis, with the full name available in a tooltip and to screen readers. The legend shall also show the total-debt marker.

```gherkin
@AC-9.1
Scenario: Legend lists the debts in creation order
  Given debts "Example Student Loan" and then "Example Mortgage", both with statements
  When the user opens the Insights tab
  Then the legend lists "Example Student Loan" and "Example Mortgage" in that order, each with its colour swatch
  And the legend shows the total-debt marker

@AC-9.2
Scenario: A debt without statements is not in the legend
  Given a debt "Example Empty" with no statements and a debt "Example Mortgage" with a statement
  When the user opens the Insights tab
  Then the legend lists only "Example Mortgage"

@AC-9.3
Scenario: Many debts wrap
  Given twelve debts with statements
  When the user opens the Insights tab on a narrow window
  Then the legend shows all twelve debts, wrapped onto more than one line
  And no debt is left out

@AC-9.4
Scenario: A long name is cut with an ellipsis
  Given a debt with a name of 100 characters and a statement
  When the user opens the Insights tab
  Then its legend item shows the name cut with an ellipsis
  And the full name is available in a tooltip and to screen readers

@AC-9.5
Scenario: A deleted debt leaves the legend
  Given the legend lists "Example Student Loan" and "Example Mortgage"
  When the user deletes "Example Student Loan" on the Overview tab and opens the Insights tab
  Then the legend lists only "Example Mortgage"
```

**REQ-10: Details of a year** *(event-driven)*
When the user hovers over or moves keyboard focus to a year, the Insights tab shall show a tooltip with the year, its rows in the order the bar shows them, top to bottom (one row per segment with the debt's swatch, full name and balance, the highest segment first, which is the last in stacking order, then the Total debt row, then the Repaid row, a dash for none), no number of statements, for the current year that it is partial and for a year without statements the note "No statements", beside the year's bar as CSD-007 REQ-14 places a tooltip, and shall give the same values to screen readers, also without a number of statements.

```gherkin
@AC-10.1
Scenario: Tooltip on hover
  Given 2026 has the segments "Example Mortgage" 150000.00 and "Example Student Loan" 8000.00, total debt 158000.00 and repaid 1500.00
  When the user hovers over the 2026 bar
  Then a tooltip shows both debts with their swatch and balance, the total debt 158000.00 and repaid 1500.00
  And it shows no number of statements

@AC-10.2
Scenario: Tooltip on keyboard focus
  Given the Insights tab shows the chart
  When the user moves focus to the 2026 bar with the Tab key
  Then the same tooltip is shown
  And the tooltip disappears when focus leaves the bar

@AC-10.3
Scenario: Accessible label
  Given 2026 has the segments "Example Mortgage" 150000.00 and "Example Student Loan" 8000.00 and repaid -200.00
  When a screen reader reads the 2026 bar
  Then it announces the year, each debt with its balance, the total debt and the amount repaid and that it is negative
  And it announces no number of statements

@AC-10.4
Scenario: Partial year
  Given the year 2026 is partial
  When the user hovers over the 2026 bar
  Then the tooltip says that the year is partial

@AC-10.5
Scenario: Repaid none is a dash
  Given 2025 is the first year of the series and has repaid none
  When the user hovers over the 2025 bar
  Then the tooltip shows a dash as repaid

@AC-10.6
Scenario: Tooltip beside the bar
  Given the chart shows the years 2024, 2025 and 2026
  When the user hovers over the 2024 bar
  Then the tooltip is shown to the right of that bar with its top level with the top of the bar
  When the user hovers over the 2026 bar and the tooltip does not fit to its right
  Then the tooltip is shown to the left of that bar and stays inside the chart area

@AC-10.7
Scenario: Year without a bar
  Given the year 2023 has no statements
  When the user hovers over its slot
  Then the tooltip is centred over that slot
  And it shows the note "No statements" and no debt rows
  And it shows no row with a number of statements

@AC-10.8
Scenario: Rows follow the bar from top to bottom
  Given 2026 has the segments "Example Mortgage" 150000.00 (first in stacking order) and "Example Student Loan" 8000.00 (second, so at the top of the bar), total debt 158000.00 and repaid 1500.00
  When the user hovers over the 2026 bar
  Then the tooltip rows are listed "Example Student Loan", "Example Mortgage", "Total debt", "Repaid" in that order

@AC-10.9
Scenario: Same order in the accessible label
  Given the same year as in AC-10.8
  When a screen reader reads the 2026 bar
  Then it announces the debts, the total debt and the amount repaid without a number of statements
```

**REQ-11: Highlight one debt** *(event-driven)*
When the mouse is over a segment or a legend item of a debt, or keyboard focus is on a legend item, the Insights tab shall keep all segments of that debt at full colour in every year and ease all other segments to 70% opacity, set the legend item and the tooltip row of that debt in the full text colour and a heavier font weight, and ease the other legend items to 70% opacity, and while no segment or legend item is hovered or focused, the Insights tab shall highlight nothing. Keyboard focus on a year highlights nothing.

```gherkin
@AC-11.1
Scenario: Hovering a segment
  Given 2025 and 2026 each have a segment of "Example Student Loan" and of "Example Mortgage"
  When the user hovers over the "Example Student Loan" segment of 2025
  Then all "Example Student Loan" segments, in 2025 and 2026, keep their full colour
  And all "Example Mortgage" segments are eased down to 70% opacity
  And the "Example Student Loan" legend item is in the full text colour and a heavier font weight
  And the other legend items are eased down to 70% opacity

@AC-11.2
Scenario: Hovering a legend item
  Given the legend lists "Example Student Loan" and "Example Mortgage"
  When the user hovers over the legend item "Example Student Loan"
  Then the segments of "Example Student Loan" keep their full colour in every year
  And the other segments are eased down to 70% opacity

@AC-11.3
Scenario: Focusing a legend item
  Given the legend lists "Example Student Loan" and "Example Mortgage"
  When the user moves keyboard focus to the legend item "Example Student Loan"
  Then the highlight is the same as when hovering over it
  And it ends when focus leaves the legend item

@AC-11.4
Scenario: Leaving the segment
  Given the user hovers over a segment
  When the user moves the mouse to empty space in the year's column
  Then no legend item is highlighted and no segment is dimmed
  And the tooltip is still shown

@AC-11.5
Scenario: Not colour alone
  Given the user hovers over a segment
  Then the highlighted legend item has a heavier font weight than the other items
  And the other segments are lighter, not differently coloured

@AC-11.6
Scenario: Keyboard focus on a year highlights nothing
  Given the Insights tab shows the chart
  When the user moves focus to the 2025 bar with the Tab key
  Then no legend item is highlighted and no segment is dimmed
```

**REQ-12: Motion and reduced motion** *(event-driven)*
When a tooltip appears or moves to another year, or the highlight of REQ-11 changes, the Insights tab shall animate the change over 120 to 150 ms with an ease-out curve, and while the user prefers reduced motion it shall show tooltips and highlight changes at once.

```gherkin
@AC-12.1
Scenario: Tooltip fades in and glides
  Given the Insights tab shows the chart
  When the user hovers over the 2025 bar and then over the 2026 bar
  Then the tooltip fades in over 120 to 150 ms while it slides 4 px towards its place
  And it glides to its new place over 120 to 150 ms instead of jumping

@AC-12.2
Scenario: Dimming eases
  Given the user hovers over a segment of "Example Student Loan"
  When the user moves the mouse away from the segment
  Then the other segments and the legend items ease back to full opacity over 120 to 150 ms

@AC-12.3
Scenario: Reduced motion
  Given the user prefers reduced motion
  When the user hovers over the 2025 bar and then over the 2026 bar
  Then each tooltip appears at once at its place without sliding or gliding
  And the dimming of segments and legend items changes at once
```

**REQ-13: Refined look and accessible segments** *(ubiquitous)*
The Insights tab shall draw the chart as CSD-010 REQ-13 describes the look of the bank chart: bars at most 50% of the width of their slot and at most 44 px wide, the top corners of the highest segment rounded by 3 px and a 1 px gap between stacked segments in place of an outline, 1 px gridlines at 55% opacity of the border colour, a 1 px zero line in the muted text colour at 70% opacity, axis and year labels in the muted text colour at regular font weight, marker values in 10 px medium weight, and a plot height of 390 px and a minimum slot width of 64 px per year, the same as the bank accounts chart (CSD-010) so that all yearly charts have one size, and tooltips of 10 px corner radius, 12 px by 14 px padding, a shadow of at least 16 px blur and at most 12% opacity, a 12 px title in semibold and row labels in the muted text colour, in light and in dark mode. Every segment shall carry an accessible label with the debt, the year and the balance.

```gherkin
@AC-13.1
Scenario: Bars
  Given the series holds the years 2024, 2025 and 2026
  When the user opens the Insights tab
  Then no bar is wider than 50% of its slot or 44 px
  And the top corners of each bar are rounded by 3 px
  And stacked segments are separated by a 1 px gap and have no outline

@AC-13.2
Scenario: Gridlines, zero line and labels
  Given the Insights tab shows the chart
  Then the gridlines are 1 px lines at 55% opacity of the border colour
  And the zero line is 1 px in the muted text colour at 70% opacity
  And the axis labels and the year labels are in the muted text colour at regular font weight

@AC-13.3
Scenario: Tooltip
  Given the user hovers over a year
  Then the tooltip has a 10 px corner radius, 12 px by 14 px padding and a soft shadow
  And its title is semibold and its row labels are in the muted text colour

@AC-13.4
Scenario: Segment labels
  Given 2026 has a segment of "Example Student Loan" with balance 8000.00
  When a screen reader reads that segment
  Then it announces "Example Student Loan", 2026 and 8000.00

@AC-13.5
Scenario: Same size as the bank accounts chart
  Given the Insights tab of the Debts page and that of the Bank accounts page
  Then both plots are 390 px high
  And both give every year a slot of at least 64 px
```

**REQ-14: No statements yet** *(state-driven)*
While at least one debt exists but no statement, the Insights tab shall show a note instead of a chart.

```gherkin
@AC-14.1
Scenario: Debts without statements
  Given a project with one debt and no statements
  When the user opens the Insights tab
  Then the note "No statements yet. The chart appears once there is a statement." is shown
  And no chart is drawn
```

**REQ-15: Current figures** *(event-driven)*
When the user opens the Insights tab, the app shall show figures computed from the project at that moment.

```gherkin
@AC-15.1
Scenario: Figures follow changes
  Given the Insights tab showed total debt 9000.00 for 2026
  And the user added a statement for 2026-10 with balance 8500.00 on the Overview tab
  When the user opens the Insights tab again
  Then the 2026 marker shows 8500.00

@AC-15.2
Scenario: Last statement deleted
  Given the Insights tab showed a chart
  And the user deleted the only statement on the Overview tab
  When the user opens the Insights tab again
  Then the note of AC-14.1 is shown and no chart is drawn

@AC-15.3
Scenario: A moved statement follows to its new year
  Given a monthly debt with a statement for 2026-01
  When the user changes its period to 2025-12 and opens the Insights tab
  Then the series holds 2025 and 2026, and the segment is in 2025
```

## 5. Interfaces and contracts

Command: `get_debt_insights()` returns the debts that have at least one statement, in creation order, and the series, oldest year first:

```json
{
  "debts": [
    { "id": "…", "name": "Example Mortgage", "colour_slot": 0 },
    { "id": "…", "name": "Example Student Loan", "colour_slot": 1 }
  ],
  "years": [
    { "year": 2025, "closing_balance": "16000.00", "repaid": null, "statement_count": 2, "is_partial": false,
      "segments": [ { "debt_id": "…", "balance": "10000.00" }, { "debt_id": "…", "balance": "6000.00" } ] },
    { "year": 2026, "closing_balance": "9500.00", "repaid": "500.00", "statement_count": 1, "is_partial": true,
      "segments": [ { "debt_id": "…", "balance": "9500.00" } ] }
  ]
}
```

`statement_count` is used to detect a year without statements and is not displayed. `debts` and `years` are empty when there are no statements. `segments` follow the creation order of the debts and are empty for a gap year, where `closing_balance` is "0.00" and `repaid` is null. Amounts are decimal strings. Rust decides "today" (local time), the years, the partial flag, the segments and the repaid amounts, and the frontend only displays what it receives. Error codes are those of CSD-011.

## 6. Constraints
- CON-1: The chart reuses the shared chart code of the frontend (`ui/shared/yearly-chart.js`) and adds no chart library or other dependency. Rationale: NG-6, and `CLAUDE.md` asks before adding dependencies.
- CON-2: The frontend displays figures computed in Rust and computes no amounts, not even the sum of the segments or the repaid amount. The only arithmetic is turning values into positions. Rationale: `CLAUDE.md` layout rule.
- CON-3: No network access. Rationale: `CLAUDE.md` Tauri security rules.
- CON-4: Debt names are drawn as plain text, never as HTML. Rationale: a name is typed by the user and must not run as markup.
- CON-5: Tests and fixtures use synthetic data only. Rationale: the repo is public.
- CON-6: The palette is the one of the bank accounts chart (CSD-010 CON-6), defined once as colour tokens per mode, and no colour of a debt is written inline in the page code. Rationale: one place to adjust, and both modes stay in step.

## 7. Verification
The series is computed in Rust, so REQ-2 to REQ-5 are proven by `cargo test`. Tests are named `test_csd012_ac_<n>_<m>_<description>` (CSD-000 REQ-6).

| Requirement | Command |
|---|---|
| REQ-2 to REQ-5, AC-15.3 (the data) | `cargo test test_csd012_` |
| REQ-1, REQ-6 to REQ-14 (all ACs), AC-15.1 to AC-15.3 (the screen) | Manual checklist: tabs, stacking, axis range, markers, colours of both modes and of the ninth debt, legend order, wrapping and ellipsis, tooltip placement, the order of its rows from top to bottom (AC-10.8, AC-10.9), no counts, its repaid row, the plot height and slot width, highlight from a segment and from the legend including keyboard, motion and reduced motion, the look, accessible labels and the empty note. There is no frontend test setup |
| One requirement, e.g. REQ-5 | `cargo test test_csd012_ac_5_` |

Done means fmt, clippy (`-D warnings`) and `cargo test --all-features` pass.

## 8. Open questions
None.

## 9. Changelog
- 0.3 (29/10/2026): The plot of every yearly chart is 390 px high (was 260 px), 50% taller. No other behaviour change.
- 0.1 (28/10/2026): initial draft, from GitHub issue 15 and the grill-me interview. It follows CSD-010 (tabs, year series, partial year, marker, tooltip, highlight, motion, look), with one segment per debt, no negative values, and the amount repaid in the tooltip.
- 0.2 (29/10/2026): No statement count is displayed any more (NG-3, REQ-10, AC-10.1, AC-10.3, AC-10.7, AC-5.1, AC-5.3, AC-5.5; the count stays in IPC, section 5). The tooltip lists its rows in the order the bar shows them, top to bottom (REQ-10, new AC-10.8 and AC-10.9, manual checklist), and an empty year shows the note "No statements". The chart has the size of the bank accounts chart: plot height 260 px, minimum slot width 64 px (REQ-13, new AC-13.5, manual checklist).

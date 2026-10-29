---
id: CSD-006
title: Income insights
status: approved
owner: Thijs Bardoel
version: 0.6
last_updated: 29/10/2026
related: [CSD-001, CSD-003, CSD-004, CSD-007]
---

# CSD-006: Income insights

## 1. Intent
The Income page lists jobs, paychecks and a table of yearly totals, but a table does not show whether income is growing. This capability adds an insights layer to the Income page: a chart of income per year, split into gross salary, holiday pay and bonus, with the year-over-year growth of total gross. It is for the owner looking back over their own employment history. It supersedes CSD-001 NG-6 for this view only.

## 2. Scope
### 2.1 In scope
- An "Insights" tab on the Income page, next to the existing content ("Overview").
- A yearly stacked bar chart over all jobs, with growth per year and a tooltip.
- Its empty states.

### 2.2 Non-goals
- NG-1: A monthly view, or any granularity other than a calendar year.
- NG-2: A job filter or a breakdown per job.
- NG-3: Summary growth figures such as the compound annual growth rate, and forecasts (CSD-001 NG-4).
- NG-4: Like-for-like comparison of the current year to date with the same months of last year.
- NG-5: Click-through, zooming, or exporting the chart.
- NG-6: Charts on the Dashboard (CSD-004 NG-2 stays) or on the Reports page.
- NG-7: A chart library or any other new dependency.

## 3. Definitions
| Term | Meaning |
|---|---|
| **Year** | A calendar year. Entries are summed per year over all jobs, as in the yearly totals (CSD-001 REQ-11). |
| **Current year** | The calendar year of today, in local time. Its figures run up to and including the current month, and are therefore partial. |
| **Growth** | The change in total gross of a year against the year before, in percent. |
| **Entry** | A paycheck of a monthly job or an annual statement of a yearly job (CSD-001). |

## 4. Requirements

### 4.0 Overview flow
```mermaid
sequenceDiagram
    actor U as User
    participant UI as Frontend (ui/)
    participant CMD as commands/
    participant DOM as domain/
    U->>UI: opens the Insights tab
    UI->>CMD: get_income_insights()
    CMD->>DOM: insights(today)
    DOM-->>CMD: one figure set per year, with growth
    CMD-->>UI: figures as decimal strings
    UI->>U: stacked bars, growth labels
```

**REQ-1: Tabs on the Income page** *(ubiquitous)*
The Income page shall offer the tabs "Overview" and "Insights", with Overview, which holds the existing content, shown first.

```gherkin
@AC-1.1
Scenario: Overview is shown first
  Given a project with one job
  When the user opens the Income page
  Then the Overview tab is selected
  And it shows the Jobs, Yearly totals and Paychecks and statements sections

@AC-1.2
Scenario: Switching to Insights
  Given the Income page shows the Overview tab
  When the user selects the Insights tab
  Then the Overview sections are hidden
  And the insights chart is shown

@AC-1.3
Scenario: No jobs yet
  Given a project without jobs
  When the user opens the Income page
  Then the prompt to add the first job is shown
  And there are no tabs
```

**REQ-2: Year series** *(ubiquitous)*
The app shall return one set of figures for every calendar year from the year of the earliest entry up to and including the current year, oldest year first, holding the sums over all jobs of gross salary, holiday pay, bonus and total gross, and the number of entries. The number of entries is part of the data only: the Insights tab does not display it.

```gherkin
@AC-2.1
Scenario: Series runs from the first entry to the current year
  Given today is 26/10/2026
  And the earliest entry is a paycheck for 2024-03
  And there are entries in 2025 and 2026
  When the insights are requested
  Then the series holds the years 2024, 2025 and 2026 in that order

@AC-2.2
Scenario: Two jobs are summed
  Given today is 26/10/2026
  And "Example Employer" has an annual statement for 2025 of salary 30000.00, holiday pay 2400.00 and bonus 0.00
  And "Example Agency" has an annual statement for 2025 of salary 5000.00, holiday pay 0.00 and bonus 1000.00
  When the insights are requested
  Then the year 2025 has gross salary 35000.00, holiday pay 2400.00, bonus 1000.00, total gross 38400.00 and 2 entries

@AC-2.3
Scenario: A single year
  Given today is 26/10/2026
  And the only entry is a paycheck for 2026-09
  When the insights are requested
  Then the series holds only the year 2026
```

**REQ-3: Years without entries** *(unwanted behaviour)*
If a year in the series has no entries, then the app shall return that year with all amounts 0.00 and 0 entries.

```gherkin
@AC-3.1
Scenario: Gap year
  Given today is 26/10/2026
  And there are entries in 2022 and 2024 but none in 2023
  When the insights are requested
  Then the year 2023 has total gross 0.00 and 0 entries
  And the series holds the years 2022, 2023, 2024, 2025 and 2026
```

**REQ-4: Current year is partial** *(state-driven)*
While a year is the current year, the app shall mark it as partial.

```gherkin
@AC-4.1
Scenario: Current year is marked
  Given today is 26/10/2026
  And there are entries in 2025 and 2026
  When the insights are requested
  Then the year 2026 is marked partial
  And the year 2025 is not
```

**REQ-5: Growth** *(ubiquitous)*
The app shall calculate the growth of a year as (total gross − total gross of the year before) ÷ total gross of the year before × 100, rounded to one decimal place with banker's rounding.

```gherkin
@AC-5.1
Scenario Outline: Growth per year
  Given today is 26/10/2026
  And the total gross of 2023 is <previous>
  And the total gross of 2024 is <current>
  When the insights are requested
  Then the growth of 2024 is "<growth>"

  Examples:
    | previous | current  | growth |
    | 32400.00 | 34480.00 | 6.4    |
    | 40000.00 | 39000.00 | -2.5   |
    | 20000.00 | 20090.00 | 0.4    |
    | 20000.00 | 20000.00 | 0.0    |
```

**REQ-6: No growth available** *(unwanted behaviour)*
If a year is the first in the series, is partial, has no entries, or follows a year with total gross 0.00, then the app shall give it no growth.

```gherkin
@AC-6.1
Scenario: First year has no growth
  Given today is 26/10/2026
  And the earliest entry is in 2024
  When the insights are requested
  Then the year 2024 has no growth

@AC-6.2
Scenario: Current year has no growth
  Given today is 26/10/2026
  And the total gross of 2025 is 34480.00 and of 2026 so far 28000.00
  When the insights are requested
  Then the year 2026 has no growth

@AC-6.3
Scenario: Year after a gap has no growth
  Given today is 26/10/2026
  And there are entries in 2022 and 2024 but none in 2023
  When the insights are requested
  Then the years 2023 and 2024 have no growth
```

**REQ-7: Chart** *(ubiquitous)*
The Insights tab shall draw one stacked bar per year in the order received, built from gross salary, holiday pay and bonus, on a y-axis from 0 to a rounded-up value of at least the largest total gross, with gridlines, compact axis labels (such as "10k"), a legend naming the three amounts, and no currency. The chart has the size and the zero line of every yearly chart (REQ-17).

```gherkin
@AC-7.1
Scenario: Stacked bars per year
  Given the series holds the years 2024, 2025 and 2026
  When the user opens the Insights tab
  Then three bars are drawn, 2024 on the left and 2026 on the right
  And each bar is stacked from gross salary, holiday pay and bonus
  And the legend names Gross salary, Holiday pay and Bonus

@AC-7.2
Scenario: Axis covers the largest year
  Given the largest total gross is 38400.00
  When the user opens the Insights tab
  Then the top of the y-axis is at least 38400.00
  And the axis starts at 0

@AC-7.3
Scenario: Gap year is an empty slot
  Given the year 2023 has total gross 0.00
  When the user opens the Insights tab
  Then 2023 has a labelled slot without a bar

@AC-7.4
Scenario: Current year is marked
  Given the year 2026 is partial
  When the user opens the Insights tab
  Then the 2026 bar has a lighter fill
  And its label reads "2026 YTD"
```

**REQ-8: Growth labels** *(ubiquitous)*
The Insights tab shall show the growth of each year with a growth value next to its bar as "+x.x%" or "−x.x%", "0.0%" for no change and "–" for no growth, using the negative colour only for decreases.

```gherkin
@AC-8.1
Scenario Outline: Growth label
  Given the growth of a year is <growth>
  When the user opens the Insights tab
  Then the label of that year reads "<label>"

  Examples:
    | growth | label |
    | 6.4    | +6.4% |
    | -2.5   | −2.5% |
    | 0.0    | 0.0%  |
    | none   | –     |

@AC-8.2
Scenario: Only decreases are red
  Given the growth of 2023 is -2.5 and of 2024 is 6.4
  When the user opens the Insights tab
  Then the label of 2023 uses the negative colour
  And the label of 2024 does not
```

**REQ-9: Details of a year** *(event-driven)*
When the user hovers over or moves keyboard focus to a year, the Insights tab shall show a tooltip with the year's gross salary, holiday pay, bonus, total gross and growth, in the order of REQ-18 and as REQ-12 lays out. The tooltip shows no count of entries, and for a year without entries it shows the note "No entries".

```gherkin
@AC-9.1
Scenario: Tooltip on hover
  Given the year 2025 has gross salary 31000.00, holiday pay 2480.00, bonus 1000.00 and total gross 34480.00
  When the user hovers over the 2025 bar
  Then a tooltip shows those four amounts and the growth of 2025
  And it shows no count of entries

@AC-9.4
Scenario: Tooltip of a year without entries
  Given the year 2023 has no entries
  When the user hovers over its slot
  Then the tooltip shows the note "No entries"
  And it shows no count of entries

@AC-9.2
Scenario: Tooltip on keyboard focus
  Given the Insights tab shows the chart
  When the user moves focus to the 2025 bar with the Tab key
  Then the same tooltip is shown
  And the tooltip disappears when focus leaves the bar

@AC-9.3
Scenario: Accessible label
  Given the year 2025 has total gross 34480.00 and growth 6.4
  When a screen reader reads the 2025 bar
  Then it announces the year, the four amounts and the growth
```

**REQ-10: No entries yet** *(state-driven)*
While the project has jobs but no entries, the Insights tab shall show a note instead of a chart.

```gherkin
@AC-10.1
Scenario: Jobs without entries
  Given a project with one job and no paychecks or statements
  When the user opens the Insights tab
  Then the note "No income yet. The chart appears once there is a paycheck or an annual statement." is shown
  And no chart is drawn
```

**REQ-11: Current figures** *(event-driven)*
When the user opens the Insights tab, the app shall show figures computed from the project at that moment.

```gherkin
@AC-11.1
Scenario: Figures follow changes
  Given the Insights tab showed total gross 34480.00 for 2025
  And the user added a bonus of 500.00 to a paycheck of 2025 on the Overview tab
  When the user opens the Insights tab again
  Then the 2025 bar shows total gross 34980.00

@AC-11.2
Scenario: Last entry deleted
  Given the Insights tab showed a chart
  And the user deleted the only entry on the Overview tab
  When the user opens the Insights tab again
  Then the note of AC-10.1 is shown and no chart is drawn
```

**REQ-12: Legend with the selected year** *(event-driven)*
When the user hovers over or moves keyboard focus to a year, the Insights tab shall show in the tooltip of that year the colour swatch of the legend on the rows of gross salary, holiday pay and bonus, and shall place the tooltip beside the year's bar, to its right with its top aligned to the top of the bar, to its left when there is no room on the right, and inside the chart area, or centred over its slot when the year has no bar. The legend in the panel header stays.

```gherkin
@AC-12.1
Scenario: Swatches match the legend
  Given the year 2025 has gross salary 31000.00, holiday pay 2480.00 and bonus 1000.00
  When the user hovers over the 2025 bar
  Then the tooltip rows Gross salary, Holiday pay and Bonus each show the swatch of the same colour as in the legend
  And the rows Total gross and Growth show no swatch

@AC-12.2
Scenario: Tooltip beside the bar
  Given the chart shows the years 2024, 2025 and 2026
  When the user hovers over the 2024 bar
  Then the tooltip is shown to the right of that bar
  And its top is level with the top of that bar

@AC-12.3
Scenario: Tooltip flips near the right edge
  Given the chart shows the years 2024, 2025 and 2026
  When the user hovers over the 2026 bar and the tooltip does not fit to its right
  Then the tooltip is shown to the left of that bar
  And it stays inside the chart area

@AC-12.4
Scenario: Year without a bar
  Given the year 2023 has total gross 0.00
  When the user hovers over its slot
  Then the tooltip is centred over that slot

@AC-12.5
Scenario: Header legend stays
  Given the Insights tab shows the chart
  When the user hovers over a year
  Then the legend in the panel header is still shown

@AC-12.6
Scenario: Keyboard focus shows the same tooltip
  Given the Insights tab shows the chart
  When the user moves focus to the 2025 bar with the Tab key
  Then the tooltip is shown beside that bar with the same swatches
```

**REQ-13: Highlight while hovering** *(event-driven)*
When the mouse is over a segment of a bar or over a legend item, or keyboard focus is on a legend item, the Insights tab shall keep all segments of that series (Gross salary, Holiday pay or Bonus) at full colour in every year and ease all other segments to 70% opacity, set the legend item of that series in the full text colour and a heavier font weight in the tooltip and in the legend, and ease the other items of the legend down to 70% opacity, and while no segment or legend item is hovered or focused, the Insights tab shall highlight nothing. Keyboard focus on a year highlights nothing.

```gherkin
@AC-13.1
Scenario: Hovering a segment
  Given the 2025 bar is stacked from gross salary, holiday pay and bonus
  When the user hovers over the holiday pay segment
  Then the Holiday pay item is set in the full text colour and a heavier font weight in the tooltip and in the legend
  And the Gross salary and Bonus items of the legend are eased down to 70% opacity
  And the holiday pay segments keep their full colour in every year
  And the gross salary and bonus segments, in every year, are eased down to 70% opacity

@AC-13.2
Scenario: Moving to another segment
  Given the user hovers over the holiday pay segment of 2025
  When the user moves the mouse to the bonus segment
  Then only the Bonus item is highlighted

@AC-13.3
Scenario: Leaving the segment
  Given the user hovers over a segment
  When the user moves the mouse to empty space in the year's column
  Then no legend item is highlighted and no segment is dimmed
  And the tooltip is still shown

@AC-13.4
Scenario: Not colour alone
  Given the user hovers over a segment
  Then the highlighted item has a heavier font weight than the other items
  And the other segments are lighter, not differently coloured
  And no outline and no background chip is drawn

@AC-13.5
Scenario: Keyboard focus on a year highlights nothing
  Given the Insights tab shows the chart
  When the user moves focus to the 2025 bar with the Tab key
  Then no legend item is highlighted

@AC-13.6
Scenario: Hovering a legend item
  Given the 2025 and 2026 bars both have gross salary, holiday pay and bonus segments
  When the user hovers over the legend item "Bonus"
  Then the bonus segments of 2025 and 2026 keep their full colour
  And the gross salary and holiday pay segments of both years are eased down to 70% opacity
  And the other legend items are eased down to 70% opacity

@AC-13.7
Scenario: Focusing a legend item
  Given the Insights tab shows the chart
  When the user moves keyboard focus to the legend item "Bonus"
  Then the highlight is the same as when hovering over it
  And it ends when focus leaves the legend item
```

**REQ-14: Motion** *(event-driven)*
When a tooltip appears or moves to another year, or the highlight of REQ-13 changes, the Insights tab shall animate the change over 120 to 150 ms with an ease-out curve: the tooltip fades in and slides 4 px towards its place, it glides to its new place when the user moves from one year to another, and the opacity of segments and legend items eases.

```gherkin
@AC-14.1
Scenario: Tooltip fades in
  Given the Insights tab shows the chart
  When the user hovers over the 2025 bar
  Then the tooltip fades in over 120 to 150 ms while it slides 4 px towards its place

@AC-14.2
Scenario: Tooltip glides between years
  Given the tooltip of 2025 is shown
  When the user moves the mouse to the 2026 bar
  Then the tooltip glides to its new place over 120 to 150 ms instead of jumping

@AC-14.3
Scenario: Dimming eases
  Given the user hovers over the holiday pay segment of 2025
  When the user moves the mouse away from the segment
  Then the other segments and the legend items ease back to full opacity over 120 to 150 ms
```

**REQ-15: Reduced motion** *(state-driven)*
While the user prefers reduced motion, the Insights tab shall show tooltips and highlight changes at once, without fading, sliding or gliding.

```gherkin
@AC-15.1
Scenario: Reduced motion
  Given the user prefers reduced motion
  When the user hovers over the 2025 bar and then over the 2026 bar
  Then each tooltip appears at once at its place without sliding or gliding
  And the dimming of segments and legend items changes at once
```

**REQ-16: Refined look** *(ubiquitous)*
The Insights tab shall draw the chart with bars at most 50% of the width of their slot and at most 44 px wide, with the top corners of the topmost segment rounded by 3 px and a 1 px gap between stacked segments in place of an outline, with 1 px gridlines at 55% opacity of the border colour and a 1 px zero line in the muted text colour at 70% opacity if one is drawn (styled as in CSD-010), with axis and year labels in the muted text colour at regular font weight, with tooltips of 10 px corner radius, 12 px by 14 px padding, a shadow of at least 16 px blur and at most 12% opacity, a 12 px title in semibold and row labels in the muted text colour, and with series colours of at most 60% HSL saturation and gain and loss colours of at most 55% HSL saturation, in light and in dark mode.

```gherkin
@AC-16.1
Scenario: Bars
  Given the series holds the years 2024, 2025 and 2026
  When the user opens the Insights tab
  Then no bar is wider than 50% of its slot or 44 px
  And the top corners of each bar are rounded by 3 px
  And stacked segments are separated by a 1 px gap and have no outline

@AC-16.2
Scenario: Gridlines and labels
  Given the Insights tab shows the chart
  Then the gridlines are 1 px lines at 55% opacity of the border colour
  And the axis labels and the year labels are in the muted text colour at regular font weight

@AC-16.3
Scenario: Tooltip
  Given the user hovers over a year
  Then the tooltip has a 10 px corner radius, 12 px by 14 px padding and a soft shadow
  And its title is semibold and its row labels are in the muted text colour

@AC-16.4
Scenario: Colours in both modes
  Given the Insights tab shows the chart
  When the app is in light mode and then in dark mode
  Then the series colours have at most 60% HSL saturation
  And the gain and loss colours have at most 55% HSL saturation
```

**REQ-17: One size for every yearly chart** *(ubiquitous)*
The Insights tab shall draw the chart with a plot height that scales with the height of the window and a minimum slot width of 64 px, with the legend (one item per series, generated by the app) above the plot, the same as the bank accounts chart (CSD-010), so that the yearly charts of all pages have the same size and behaviour.

```gherkin
@AC-17.1
Scenario: Same plot height and slot width as the bank accounts chart
  Given the Insights tab shows the chart
  Then the plot height scales with the height of the window
  And each year's slot is at least 64 px wide
  And the zero line, if drawn, is styled as on the bank accounts chart

@AC-17.2
Scenario: Many years scroll
  Given the series holds more years than fit at 64 px per slot
  When the user opens the Insights tab
  Then the chart scrolls sideways and no slot is narrower than 64 px

@AC-17.3
Scenario: Legend lists the series
  Given the Insights tab shows the chart
  Then the legend lists Gross salary, Holiday pay and Bonus, each with its colour swatch
```

**REQ-18: Tooltip rows in the order of the bar** *(ubiquitous)*
The tooltip of a year shall list its rows in the order the bar shows them, top to bottom: Bonus, Holiday pay and Gross salary, followed by Total gross and Growth.

```gherkin
@AC-18.1
Scenario: Rows read like the bar
  Given the year 2025 has gross salary 31000.00, holiday pay 2480.00 and bonus 1000.00
  When the user hovers over the 2025 bar
  Then the tooltip rows are, from top to bottom, Bonus, Holiday pay, Gross salary, Total gross and Growth
```

## 5. Interfaces and contracts

Command: `get_income_insights()` returns the series, oldest year first:

```json
{
  "years": [
    { "year": 2024, "gross_salary": "30000.00", "holiday_pay": "2400.00", "bonus": "0.00", "total_gross": "32400.00", "entry_count": 12, "is_partial": false, "growth_percent": null },
    { "year": 2025, "gross_salary": "31000.00", "holiday_pay": "2480.00", "bonus": "1000.00", "total_gross": "34480.00", "entry_count": 12, "is_partial": false, "growth_percent": "6.4" },
    { "year": 2026, "gross_salary": "26000.00", "holiday_pay": "0.00", "bonus": "0.00", "total_gross": "26000.00", "entry_count": 10, "is_partial": true, "growth_percent": null }
  ]
}
```

`entry_count` stays in the payload (Rust-side use and tests) but the frontend does not display it, not in the tooltip and not elsewhere. `years` is empty when there are no entries. `growth_percent` is a decimal string or `null`. Amounts are decimal strings. Rust decides "today" (local time), the years, the partial flag and the growth, and the frontend only displays what it receives.

## 6. Constraints
- CON-1: Figures and growth are computed in Rust and displayed as received; the frontend only scales values to positions on the chart. Rationale: `CLAUDE.md` layout rule.
- CON-2: Amounts and growth use `rust_decimal::Decimal`. Rationale: `CLAUDE.md` money rules.
- CON-3: The chart is plain SVG drawn in `ui/`, with no chart library, no new crate and no network access. Rationale: `CLAUDE.md` dependency and Tauri security rules.
- CON-4: Chart colours come from CSS variables and are legible in light and dark mode. Rationale: the app already supports both.
- CON-5: The Overview tab behaves as before (CSD-001 is unchanged apart from NG-6). Rationale: this capability adds a view and changes no income behaviour.
- CON-6: Tests and fixtures use synthetic data only. Rationale: the repo is public.
- CON-7: The highlight of REQ-13 is drawn with CSS classes and carries no meaning by colour alone. Rationale: readers who cannot tell the colours apart, and no new dependency.
- CON-8: Motion is drawn with CSS transitions only and respects the reduced-motion preference of REQ-15. Rationale: no new dependency, and users who ask for less motion.

## 7. Verification
The figures are proven by `cargo test`. Tests are named `test_csd006_ac_<n>_<m>_<description>` (CSD-000 REQ-6).

| Requirement | Command |
|---|---|
| REQ-2 to REQ-6 (data) | `cargo test test_csd006_` |
| AC-1.1 to AC-1.3, REQ-7, REQ-8, REQ-9, REQ-10, REQ-11, REQ-12, REQ-13, REQ-14, REQ-15, REQ-16, REQ-17, REQ-18 (display) | Manual checklist: motion of the tooltip and the dimming, reduced motion, bar shape, gridlines, labels, tooltip look and colours in both modes, tabs, bars and legend, axis, gap and YTD years, growth labels and colours, tooltip by mouse and keyboard, swatches and placement of the tooltip, chart size (plot height scaling with the window, 64 px slots) and legend, tooltip row order and "No entries" note without counts, legend and segment highlight on hover and legend-item focus across all years, no highlight on keyboard focus, empty note, refresh after a change, light and dark mode. There is no frontend test setup |

Done means fmt, clippy (`-D warnings`) and `cargo test --all-features` pass.

## 8. Open questions
None.

## 9. Changelog
- 0.6 (29/10/2026): The plot height scales with the height of the window instead of being a fixed 390 px: with the page scrolled to the top the chart ends at the bottom of the window, and the plot is never lower than 260 px. It is recomputed when the window is resized.
- 0.5 (29/10/2026): The plot of every yearly chart is 390 px high (was 260 px), 50% taller. No other behaviour change.
- 0.4 (29/10/2026): The yearly charts share one size and behaviour with the bank accounts chart (CSD-010). New REQ-17 (plot height 260 px, minimum slot width 64 px, zero line, generated legend) and REQ-18 (tooltip rows in the order of the bar). REQ-13 now highlights a series in every year and also on hovering or focusing a legend item (AC-13.1 amended, AC-13.6 and AC-13.7 added). The tooltip shows no entry count and the note "No entries" for an empty year (REQ-9, AC-9.1, AC-9.4). `entry_count` stays in the payload but is not displayed. All new ACs are on the manual checklist.
- 0.3 (27/10/2026): Quieter and more refined charts. REQ-13 (AC-13.1, AC-13.4) replaces the chip and outline by a full-colour segment, eased dimming of the rest and a heavier legend label. New REQ-14 motion, REQ-15 reduced motion, REQ-16 refined look (AC-14.1 to AC-16.4), CON-8. The tooltip placement of REQ-12 is unchanged. Approved by the owner. Same change for the investment chart in CSD-007 0.4.
- 0.2 (27/10/2026): Legend with the selected year and highlight on hover. New REQ-12 (swatches in the tooltip, tooltip beside the bar, AC-12.1 to AC-12.6) and REQ-13 (AC-13.1 to AC-13.5), CON-7. REQ-9 points to REQ-12. Approved by the owner. Same change for the investment chart in CSD-007 0.2.
- 0.1 (26/10/2026): initial draft.

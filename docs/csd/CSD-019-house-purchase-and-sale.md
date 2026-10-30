---
id: CSD-019
title: House purchase and sale
status: approved
owner: Thijs Bardoel
version: 0.2
last_updated: 30/10/2026
related: [CSD-000, CSD-002, CSD-003, CSD-015, CSD-016, CSD-017, CSD-018, CSD-020]
---

# CSD-019: House purchase and sale

## 1. Intent
Houses (CSD-015) are a value history only: the app does not know when a house was bought, for how much, or whether it was sold. A sold house keeps counting in the total value, the yearly values, the net worth and the ratios wherever its last valuation falls. This capability gives every house one purchase (date and amount) and at most one sale (date and amount), so the app knows which houses are held and in which years. It is for a single person keeping a private, local history. The amounts are a record: they do not change net worth or cash. From them the app derives the result of a sale. It follows GitHub issue 27 and the grill-me interview, and it replaces the non-goals of CSD-015 that excluded a purchase price, a sold state and purchase and sale transactions.

## 2. Scope
### 2.1 In scope
- A purchase date and purchase amount on every house, required for a new house.
- An optional sale: sell date and sell amount, set together, and removable by clearing both.
- Date and amount rules, and the holding window that limits a house's valuations.
- The held or sold state and the result of a sale, shown in the Houses view.
- The effect on the total value (held houses only) and on the yearly values (houses held at the end of the year), which net worth, the house insights and the ratios inherit.
- Opening project files whose houses have no purchase fields.

### 2.2 Non-goals
- NG-1: More than one purchase or sale per house: top-ups, partial sales, quantities and lots. A top-up is a new house.
- NG-2: Fees, tax and cost-basis methods.
- NG-3: Percentage and annualised returns. Only the amount difference is shown.
- NG-4: Any cash or net-worth effect of the amounts: the sell amount is not added to a bank account or to net worth, which stays valuation-based (CSD-017). Settling the debts linked to a sold house (CSD-020) is separate: it follows the debts, not the amounts.
- NG-5: Creating a valuation automatically at the purchase or the sale date.
- NG-6: Importing or back-filling purchase dates for existing houses.
- NG-7: Archiving, filtering or hiding sold houses. They stay listed.

## 3. Definitions
| Term | Meaning |
|---|---|
| **Purchase** | The purchase date and the purchase amount of a house. Required for a new house. |
| **Sale** | The sell date and the sell amount of a house. Both or neither. |
| **Held** | A house without a sale. **Sold** is a house with one. |
| **Holding window** | From the period of the purchase date to the period of the sell date, both included, where *period* is the month (`YYYY-MM`) of a monthly house and the year (`YYYY`) of a yearly house. It has no end while the house is held. |
| **Held at the end of a year** | Purchased on or before 31/12 of that year, and not sold on or before 31/12 of that year. A house without a purchase (REQ-6) counts as purchased before any year. |
| **Result** | The sell amount minus the purchase amount of a sold house. Positive for a gain, negative for a loss. It is derived and never stored. |
| **Legacy house** | A house from a project file made before this capability: it has no purchase. |

## 4. Requirements

> CSD-020 extends REQ-2 (record and undo a sale) for a house with linked debts: saving a sale asks Repaid or Cancelled per open debt, and undoing it needs a confirmation. Where they differ, CSD-020 applies.

### 4.0 Overview flow
```mermaid
sequenceDiagram
    actor U as User
    participant UI as Frontend (ui/)
    participant CMD as commands/
    participant DOM as domain/
    U->>UI: fills in the house form (dates dd/mm/yyyy, amounts)
    UI->>CMD: create_house / update_house(input)
    CMD->>DOM: validate dates, amounts, holding window
    DOM-->>CMD: House or validation error
    CMD-->>UI: serialised response (dates ISO 8601, amounts as decimal strings)
    UI->>CMD: list_houses() / yearly_house_values()
    CMD->>DOM: held state, result, total, yearly values
    DOM-->>CMD: figures
    CMD-->>UI: serialised response
    Note over UI,DOM: change is in memory only until saved (see CSD-002)
```

### 4.1 Purchase and sale

**REQ-1: Purchase on a new house** *(event-driven)*
When the user adds a house, the app shall require a purchase date and a purchase amount and store them with the house.

```gherkin
@AC-1.1
Scenario: House with a purchase
  Given the current date is 29/10/2026
  When the user adds a house "Example House" with purchase date 15/03/2024 and purchase amount 250000.00
  Then the house has purchase date 15/03/2024 and purchase amount 250000.00
  And it has no sale and is held

@AC-1.2
Scenario Outline: A missing purchase field is rejected
  When the user adds a house "Example House" with purchase date "<date>" and purchase amount "<amount>"
  Then it is rejected with error code "<code>"
  And no house exists

  Examples:
    | date       | amount    | code                           |
    | (empty)    | 250000.00 | HOUSE_PURCHASE_DATE_REQUIRED   |
    | 15/03/2024 | (empty)   | HOUSE_PURCHASE_AMOUNT_REQUIRED |
```

**REQ-2: Record and undo a sale** *(event-driven)*
When the user saves a house with a sell date and a sell amount, the app shall store the sale, and when the user saves it with both emptied, the app shall remove the sale and the house is held again.

```gherkin
@AC-2.1
Scenario: Sell a house
  Given the current date is 29/10/2026
  And a held house "Example House" bought on 15/03/2024 for 250000.00
  When the user sets sell date 10/06/2026 and sell amount 300000.00
  Then the house is sold on 10/06/2026 for 300000.00

@AC-2.2
Scenario: Undo a sale
  Given a house "Example House" sold on 10/06/2026 for 300000.00
  When the user empties the sell date and the sell amount and saves
  Then the house has no sale and is held

@AC-2.3
Scenario: A sale on the day of purchase is allowed
  Given a held house "Example Cabin" bought on 01/09/2026 for 10000.00
  When the user sets sell date 01/09/2026 and sell amount 9500.00
  Then the house is sold on 01/09/2026
```

**REQ-3: Date rules** *(unwanted behaviour)*
If a purchase or sell date is not a valid date, is later than today, or the sell date is earlier than the purchase date, then the app shall reject the house or edit with the error code of the violated rule, without storing or changing anything.

```gherkin
@AC-3.1
Scenario Outline: Invalid dates rejected
  Given the current date is 29/10/2026
  When the user adds a house "Example House" with purchase date "<purchase>" and purchase amount 250000.00 and sell date "<sell>" and sell amount 1000.00
  Then it is rejected with error code "<code>"
  And no house exists

  Examples:
    | purchase   | sell       | code                         |
    | 31/02/2024 | (empty)    | HOUSE_DATE_INVALID           |
    | 2024-03-15 | (empty)    | HOUSE_DATE_INVALID           |
    | 30/10/2026 | (empty)    | HOUSE_DATE_IN_FUTURE         |
    | 15/03/2024 | 30/10/2026 | HOUSE_DATE_IN_FUTURE         |
    | 15/03/2024 | 14/03/2024 | HOUSE_SELL_BEFORE_PURCHASE   |

@AC-3.2
Scenario: Today is accepted
  Given the current date is 29/10/2026
  When the user adds a house "Example House" with purchase date 29/10/2026 and purchase amount 250000.00
  Then the house exists
```

**REQ-4: Amount rules** *(unwanted behaviour)*
If a purchase or sell amount is not a plain number, is negative, has more than two decimal places or exceeds 1,000,000,000.00, then the app shall reject the house or edit with the error code of the violated rule, without rounding or storing it. An amount of 0.00 is valid.

```gherkin
@AC-4.1
Scenario Outline: Invalid amounts rejected
  Given a held house "Example House" bought on 15/03/2024 for 250000.00
  When the user sets sell date 10/06/2026 and sell amount "<amount>"
  Then it is rejected with error code "<code>"
  And the house is still held

  Examples:
    | amount         | code                    |
    | abc            | HOUSE_AMOUNT_INVALID    |
    | 1e3            | HOUSE_AMOUNT_INVALID    |
    | -0.01          | HOUSE_AMOUNT_NEGATIVE   |
    | 1000.999       | HOUSE_AMOUNT_PRECISION  |
    | 1000000000.01  | HOUSE_AMOUNT_TOO_LARGE  |

@AC-4.2
Scenario: A worthless sale and a free purchase are valid
  When the user adds a house "Example Guesthouse" with purchase date 15/03/2024 and purchase amount 0.00 and sell date 10/06/2026 and sell amount 0.00
  Then the house is sold with result 0.00
```

**REQ-5: A sale is complete** *(unwanted behaviour)*
If the user saves a house with a sell date but no sell amount, or a sell amount but no sell date, then the app shall reject it with the error code `HOUSE_SALE_INCOMPLETE`.

```gherkin
@AC-5.1
Scenario Outline: Half a sale rejected
  Given a held house "Example House" bought on 15/03/2024 for 250000.00
  When the user saves it with sell date "<date>" and sell amount "<amount>"
  Then it is rejected with error code "HOUSE_SALE_INCOMPLETE"
  And the house is still held

  Examples:
    | date       | amount    |
    | 10/06/2026 | (empty)   |
    | (empty)    | 300000.00 |
```

**REQ-6: Legacy houses** *(event-driven)*
When the user opens a project file whose houses have no purchase, the app shall open it without error and show those houses as held with a dash for the purchase date and amount, and when the user saves an edit of a legacy house, the app shall require the purchase under REQ-1.

```gherkin
@AC-6.1
Scenario: Legacy file opens
  Given a project file with a house "Example House" and no purchase fields
  When the user opens it
  Then the house exists, is held, and shows a dash as purchase date and purchase amount
  And no error is shown

@AC-6.2
Scenario: Editing a legacy house needs the purchase
  Given a legacy house "Example House"
  When the user renames it to "Example Home" without a purchase
  Then the edit is rejected with error code "HOUSE_PURCHASE_DATE_REQUIRED"
  And the house is still named "Example House"

@AC-6.3
Scenario: A legacy house is complete once the purchase is filled in
  Given a legacy house "Example House"
  When the user saves it with purchase date 15/03/2024 and purchase amount 250000.00
  Then the house has that purchase

@AC-6.4
Scenario: A legacy house has no lower bound on valuations
  Given a legacy monthly house "Example House"
  When the user adds a valuation for 2019-01 with value 200000.00
  Then the valuation for 2019-01 exists
```

### 4.2 Holding window

**REQ-7: Valuations lie in the holding window** *(unwanted behaviour)*
If the period of an added or edited valuation is earlier than the period of the purchase date or later than the period of the sell date, then the app shall reject it with the error code `HOUSE_VALUATION_OUTSIDE_HOLDING`, leaving the valuations unchanged.

```gherkin
@AC-7.1
Scenario Outline: Period outside the window rejected
  Given a monthly house "Example House" bought on 15/03/2024 and sold on 10/06/2026
  When the user adds a valuation for "<period>" with value 1000.00
  Then <outcome>

  Examples:
    | period  | outcome                                                          |
    | 2024-02 | it is rejected with error code "HOUSE_VALUATION_OUTSIDE_HOLDING" |
    | 2024-03 | the valuation for 2024-03 exists                                 |
    | 2026-06 | the valuation for 2026-06 exists                                 |
    | 2026-07 | it is rejected with error code "HOUSE_VALUATION_OUTSIDE_HOLDING" |

@AC-7.2
Scenario: A yearly house is compared by year
  Given a yearly house "Example Lodge" bought on 15/03/2024
  When the user adds a valuation for 2023 with value 1000.00
  Then it is rejected with error code "HOUSE_VALUATION_OUTSIDE_HOLDING"
  When the user adds a valuation for 2024 with value 1000.00
  Then the valuation for 2024 exists

@AC-7.3
Scenario: Moving a valuation out of the window is rejected
  Given a monthly house bought on 15/03/2024 with a valuation for 2024-05 with value 1000.00
  When the user changes its period to 2024-01
  Then the edit is rejected with error code "HOUSE_VALUATION_OUTSIDE_HOLDING"
  And the valuation is still for 2024-05
```

**REQ-8: Dates may not orphan valuations** *(unwanted behaviour)*
If the user saves a purchase date or sell date so that an existing valuation of the house falls outside the holding window, then the app shall reject the whole edit with the error code `HOUSE_DATES_EXCLUDE_VALUATIONS`.

```gherkin
@AC-8.1
Scenario Outline: Narrowing the window is rejected
  Given a monthly house "Example House" bought on 15/03/2024 with a valuation for 2024-05 and one for 2026-09
  When the user saves it with <change>
  Then the edit is rejected with error code "HOUSE_DATES_EXCLUDE_VALUATIONS"
  And the dates of the house are unchanged

  Examples:
    | change                                    |
    | purchase date 01/06/2024                  |
    | sell date 10/06/2026 and sell amount 1.00 |

@AC-8.2
Scenario: Widening the window is accepted
  Given a monthly house bought on 15/03/2024 with a valuation for 2024-05
  When the user saves it with purchase date 01/01/2024
  Then the house has purchase date 01/01/2024
```

### 4.3 Views and figures

**REQ-9: Held or sold, and the result** *(ubiquitous)*
The app shall show for each house its purchase date and amount, and either "Held" or "Sold on dd/mm/yyyy" with the sell amount and, computed in Rust, the result. Sold houses shall stay in the list in the order of CSD-015 REQ-18.

```gherkin
@AC-9.1
Scenario: Held house
  Given a house "Example House" bought on 15/03/2024 for 250000.00 and not sold
  Then its row shows purchase "15/03/2024" and "250000.00" and "Held"

@AC-9.2
Scenario Outline: Sold house with its result
  Given a house "Example House" bought on 15/03/2024 for 250000.00 and sold on 10/06/2026 for <sell>
  Then its row shows "Sold on 10/06/2026" with sell amount <sell> and result <result>

  Examples:
    | sell      | result     |
    | 300000.00 | 50000.00   |
    | 240000.00 | -10000.00  |
    | 250000.00 | 0.00       |

@AC-9.3
Scenario: A sold house stays in the list
  Given a sold house "Example House" and a held house "Example Lodge"
  Then both are listed alphabetically
```

**REQ-10: Total value counts held houses** *(ubiquitous)*
The app shall compute the total value of CSD-015 REQ-17 over held houses only. A sold house keeps showing its latest value on its row and is not part of the total.

```gherkin
@AC-10.1
Scenario: Sold house not in the total
  Given a held house "Example Lodge" with latest valuation 2026-09 (value 50000.00)
  And a sold house "Example House" with latest valuation 2026-05 (value 300000.00)
  Then the total value is 50000.00
  And "Example House" shows latest value 300000.00 for 2026-05

@AC-10.2
Scenario: Selling a house lowers the total
  Given held houses "Example House" (latest value 300000.00) and "Example Lodge" (latest value 50000.00)
  When the user sells "Example House" on 10/06/2026 for 310000.00
  Then the total value is 50000.00
```

**REQ-11: Yearly figures count houses held at year end** *(ubiquitous)*
The app shall include a house in the closing value and the change of a calendar year only if it was held at the end of that year, in the yearly values (CSD-015 REQ-20) and in the figures built on them: the house insights (CSD-016), the net worth (CSD-017) and the ratios (CSD-018). A year is listed in the yearly values only if at least one house held at its end has a valuation in it. Values are still not carried forward.

```gherkin
@AC-11.1
Scenario: A sold house stops counting in its sale year
  Given a monthly house "Example House" bought on 15/03/2024 and sold on 10/06/2026
  And valuations 2024-12 (value 200000.00), 2025-12 (value 210000.00) and 2026-03 (value 215000.00)
  When the user views the yearly values
  Then 2025 shows closing value 210000.00, change 10000.00
  And 2024 shows closing value 200000.00, change none
  And 2026 is not listed

@AC-11.2
Scenario: The insights show zero from the sale year
  Given the same house and the current date 29/10/2026
  When the user views the house insights
  Then 2026 shows closing value 0.00

@AC-11.3
Scenario: Year-end boundary
  Given a monthly house with a valuation 2026-12 (value 1000.00)
  And the house is sold on <sell>
  Then 2026 <outcome>

  Examples:
    | sell       | outcome                           |
    | 31/12/2026 | is not listed in the yearly values |
    | (none)     | shows closing value 1000.00       |

@AC-11.4
Scenario: Other houses are unaffected
  Given a sold house "Example House" and a held house "Example Lodge" with a valuation 2026-03 (value 50000.00)
  When the user views the yearly values
  Then 2026 shows closing value 50000.00

@AC-11.5
Scenario: A purchase after year end is not counted in that year
  Given a monthly house bought on 15/01/2026 with a valuation 2026-03 (value 1000.00)
  Then 2025 is not listed in the yearly values

@AC-11.6
Scenario: Net worth drops the sold house
  Given a monthly house sold on 10/06/2026 with valuation 2025-12 (value 210000.00)
  When the user views the net worth
  Then the houses figure of 2025 is 210000.00
  And the houses figure of 2026 is 0.00
```

**REQ-12: The house form** *(event-driven)*
When the user opens the add or edit house form, the app shall offer the purchase date, purchase amount, sell date and sell amount next to the name and the valuation period, with dates typed as dd/mm/yyyy, and shall do all date and amount validation in Rust.

```gherkin
@AC-12.1
Scenario: Fields in the form
  When the user opens the add house form
  Then it asks for Name, Valuation period, "Purchase date (dd/mm/yyyy)", "Purchase amount", "Sell date (dd/mm/yyyy)" and "Sell amount"
  And the sell fields are marked optional

@AC-12.2
Scenario: Editing shows the stored values
  Given a house sold on 10/06/2026 for 300000.00
  When the user opens it for editing
  Then the form shows sell date "10/06/2026" and sell amount "300000.00"

@AC-12.3
Scenario: An edit is an unsaved change
  Given a saved project with a held house
  When the user sells it
  Then the unsaved indicator is shown
```

### 4.4 Saving, loading and currency

**REQ-13: Purchase and sale are part of the project file** *(event-driven)*
When the user saves and later opens a project, the app shall restore the purchase and the sale of every house exactly.

```gherkin
@AC-13.1
Scenario: Round trip
  Given a house "Example House" bought on 15/03/2024 for 250000.00 and sold on 10/06/2026 for 300000.00
  When the user saves the project and opens it again
  Then the house has the same purchase and sale

@AC-13.2
Scenario: A damaged sale is an invalid file
  Given a project file with a house that has a sell date but no sell amount
  When the user opens it
  Then it is rejected with the error code "PROJECT_INVALID_FILE"

@AC-13.3
Scenario: A valuation outside the window is an invalid file
  Given a project file with a house bought on 15/03/2024 and a valuation for 2023-12
  When the user opens it
  Then it is rejected with the error code "PROJECT_INVALID_FILE"
```

**REQ-14: Purchase and sale do not count for the currency confirmation** *(ubiquitous)*
The app shall leave the purchase and sale amounts out of the entry count that triggers the currency confirmation of CSD-003 REQ-5.

```gherkin
@AC-14.1
Scenario: Amounts alone need no confirmation
  Given a project in "EUR" with a sold house and no valuations
  When the user chooses the currency "USD"
  Then the currency changes to "USD" without a confirmation
```

## 5. Interfaces and contracts

```mermaid
erDiagram
    HOUSE ||--o{ HOUSE_VALUATION : has
    HOUSE {
        HouseId id
        string name "CSD-015"
        PayPeriod valuation_period "CSD-015"
        u8 colour_slot "CSD-015"
        Date purchase_date "optional in the file (legacy), required on create and on save"
        Decimal purchase_amount "0.00 to 1000000000.00, two decimals"
        Date sell_date "optional, set together with sell_amount"
        Decimal sell_amount "optional, 0.00 to 1000000000.00"
    }
```

The result, the held or sold state and the held-at-year-end rule are derived (REQ-9, REQ-11) and not stored. In the project file the four fields are optional with a default of none, so existing files load and `format_version` stays 1. Dates in the file and over IPC are ISO 8601 (`YYYY-MM-DD`) and amounts decimal strings (CSD-000 CON-7).

Commands: `create_house` and `update_house` take the four extra fields. `list_houses()` adds, per house, the purchase, the sale, held or sold, and the result or null, and its total covers held houses only. `yearly_house_values()`, `get_house_insights()` and the net-worth and ratios commands apply REQ-11. Project load validates REQ-13.

Error codes (code plus safe message): `HOUSE_PURCHASE_DATE_REQUIRED`, `HOUSE_PURCHASE_AMOUNT_REQUIRED`, `HOUSE_DATE_INVALID`, `HOUSE_DATE_IN_FUTURE`, `HOUSE_SELL_BEFORE_PURCHASE`, `HOUSE_SALE_INCOMPLETE`, `HOUSE_AMOUNT_INVALID`, `HOUSE_AMOUNT_NEGATIVE`, `HOUSE_AMOUNT_PRECISION`, `HOUSE_AMOUNT_TOO_LARGE`, `HOUSE_VALUATION_OUTSIDE_HOLDING`, `HOUSE_DATES_EXCLUDE_VALUATIONS`.

## 6. Constraints
- CON-1: Amounts use `rust_decimal::Decimal` and reuse the value rules of CSD-015 REQ-10. Rationale: `CLAUDE.md` money rules.
- CON-2: The frontend computes no dates, held state, result or totals. Rationale: `CLAUDE.md` layout rule.
- CON-3: The result and the held-at-year-end rule are computed in `domain/houses` so net worth and ratios inherit them, and are never stored. Rationale: CSD-015 CON-9.
- CON-4: Amounts and dates are not logged at `info` or above, and tests use synthetic data. Rationale: `CLAUDE.md` privacy rules, public repo.
- CON-5: No network access. Rationale: `CLAUDE.md` Tauri security rules.

## 7. Verification
All ACs are proven by `cargo test`, with domain logic in `domain/`. Tests are named `test_csd019_ac_<n>_<m>_<description>` (CSD-000 REQ-6).

| Requirement | Command |
|---|---|
| REQ-1 to REQ-11, REQ-13, REQ-14 (data) | `cargo test test_csd019_` |
| One requirement, e.g. REQ-11 | `cargo test test_csd019_ac_11_` |
| REQ-12, AC-6.1 and AC-9.1 to AC-9.3 (display) | Manual checklist: form fields and labels, dd/mm/yyyy input, dashes for legacy houses, "Held" and "Sold on" text, result with sign, unsaved indicator. There is no frontend test setup |

Done means fmt, clippy (`-D warnings`) and `cargo test --all-features` pass. CSD-015 REQ-17 and REQ-20 tests are updated where REQ-10 and REQ-11 change their figures.

## 8. Open questions
None.

## 9. Changelog
- 0.2 (30/10/2026): Assets are renamed to houses. NG-4 now excludes only the cash and net-worth effect of the amounts: selling a house that has linked debts also settles them (CSD-020), which extends REQ-2. Back to `draft` until re-approved (GitHub issue 31).
- 0.1 (29/10/2026): initial draft, from GitHub issue 27 and the grill-me interview. It supersedes CSD-015 NG-1 (purchase price), NG-6 (sold state) and NG-8 (purchase and sale transactions), and extends CSD-015 REQ-1, REQ-4, REQ-7, REQ-13, REQ-17, REQ-20 and REQ-24.

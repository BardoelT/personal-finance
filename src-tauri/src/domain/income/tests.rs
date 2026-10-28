use super::*;

fn month(text: &str) -> YearMonth {
    YearMonth::parse(text).unwrap()
}

fn today() -> YearMonth {
    month("2026-10")
}

fn input(employer: &str, start: &str, end: &str) -> JobInput {
    JobInput {
        employer: employer.to_owned(),
        title: None,
        start_date: Some(start.to_owned()),
        end_date: Some(end.to_owned()),
        pay_period: None,
    }
}

fn job_with(income: &mut Income, start: &str, end: &str) -> JobId {
    income
        .create_job(&input("Example Employer", start, end))
        .unwrap()
        .id
}

fn amounts(gross: &str, holiday: &str, bonus: &str) -> Amounts {
    Amounts::parse(gross, holiday, bonus).unwrap()
}

fn add(income: &mut Income, job: JobId, m: &str, gross: &str) -> Paycheck {
    income
        .add_paycheck(job, month(m), amounts(gross, "", ""), today())
        .unwrap()
}

// ---- REQ-1: add a job ----

#[test]
fn test_csd001_ac_1_1_job_created_with_defaults() {
    let mut income = Income::default();
    let job = income
        .create_job(&JobInput {
            employer: "Example Employer".into(),
            start_date: Some("01/03/2024".into()),
            ..JobInput::default()
        })
        .unwrap();
    assert_eq!(income.jobs().len(), 1);
    assert_eq!(job.employer, "Example Employer");
    assert_eq!(job.start_date.to_string(), "2024-03-01");
    assert_eq!(job.title, None);
    assert_eq!(job.end_date, None);
}

#[test]
fn test_csd001_ac_1_2_invalid_job_is_rejected() {
    let cases = [
        ("", "01/03/2024", "", DomainError::JobEmployerRequired),
        ("   ", "01/03/2024", "", DomainError::JobEmployerRequired),
        ("Example Employer", "", "", DomainError::JobStartRequired),
        (
            "Example Employer",
            "01/03/2024",
            "29/02/2024",
            DomainError::JobEndBeforeStart,
        ),
        (
            "Example Employer",
            "01-03-2024",
            "",
            DomainError::JobDateInvalid,
        ),
    ];
    for (employer, start, end, expected) in cases {
        let mut income = Income::default();
        let error = income.create_job(&input(employer, start, end)).unwrap_err();
        assert_eq!(error, expected, "{employer:?} {start:?} {end:?}");
        assert_eq!(income.jobs().len(), 0);
    }
}

#[test]
fn test_csd001_ac_1_2_error_codes_match_the_spec() {
    assert_eq!(
        DomainError::JobEmployerRequired.code(),
        "JOB_EMPLOYER_REQUIRED"
    );
    assert_eq!(DomainError::JobStartRequired.code(), "JOB_START_REQUIRED");
    assert_eq!(
        DomainError::JobEndBeforeStart.code(),
        "JOB_END_BEFORE_START"
    );
}

#[test]
fn overlong_text_is_rejected() {
    let mut income = Income::default();
    let long = "x".repeat(201);
    let error = income
        .create_job(&input(&long, "01/03/2024", ""))
        .unwrap_err();
    assert_eq!(error, DomainError::JobFieldTooLong);
}

// ---- REQ-2: edit a job ----

#[test]
fn test_csd001_ac_2_1_dates_change_freely_with_paychecks_present() {
    let mut income = Income::default();
    let id = job_with(&mut income, "01/03/2024", "");
    add(&mut income, id, "2024-03", "3000.00");
    let updated = income
        .update_job(id, &input("Example Employer", "01/06/2024", "31/01/2025"))
        .unwrap();
    assert_eq!(updated.start_date.to_string(), "2024-06-01");
    assert_eq!(
        updated.end_date.map(|d| d.to_string()).as_deref(),
        Some("2025-01-31")
    );
    assert_eq!(income.entry_count(id), 1);
}

#[test]
fn test_csd001_ac_2_2_end_before_start_is_rejected_on_edit() {
    let mut income = Income::default();
    let id = job_with(&mut income, "01/03/2024", "");
    let error = income
        .update_job(id, &input("Example Employer", "01/03/2024", "01/02/2024"))
        .unwrap_err();
    assert_eq!(error.code(), "JOB_END_BEFORE_START");
    assert_eq!(income.jobs()[0].end_date, None);
}

#[test]
fn editing_an_unknown_job_is_rejected() {
    let mut income = Income::default();
    let error = income
        .update_job(
            JobId::generate(),
            &input("Example Employer", "01/03/2024", ""),
        )
        .unwrap_err();
    assert_eq!(error, DomainError::JobNotFound);
}

// ---- REQ-4: delete a job ----

#[test]
fn test_csd001_ac_4_1_deletion_cascades() {
    let mut income = Income::default();
    let id = job_with(&mut income, "01/03/2024", "");
    let other = job_with(&mut income, "01/01/2020", "");
    for m in ["2026-07", "2026-08", "2026-09"] {
        add(&mut income, id, m, "3000.00");
    }
    add(&mut income, other, "2026-09", "1000.00");
    income.delete_job(id).unwrap();
    assert_eq!(income.jobs().len(), 1);
    assert_eq!(income.entry_count(id), 0);
    assert_eq!(income.entry_count(other), 1);
    assert_eq!(income.paychecks(id).unwrap_err(), DomainError::JobNotFound);
}

// AC-4.2 (cancelled deletion) is a UI confirmation: the domain only deletes
// when asked to, covered by the manual checklist.

// ---- REQ-5: add a paycheck ----

#[test]
fn test_csd001_ac_5_1_paycheck_with_defaults() {
    let mut income = Income::default();
    let id = job_with(&mut income, "01/03/2024", "");
    let paycheck = add(&mut income, id, "2026-09", "3000.00");
    assert_eq!(paycheck.amounts.gross_salary().to_string(), "3000.00");
    assert_eq!(paycheck.amounts.holiday_pay().to_string(), "0.00");
    assert_eq!(paycheck.amounts.bonus().to_string(), "0.00");
    assert_eq!(paycheck.amounts.total_gross().to_string(), "3000.00");
}

#[test]
fn test_csd001_ac_5_2_month_before_job_start_is_accepted() {
    let mut income = Income::default();
    let id = job_with(&mut income, "01/03/2024", "");
    add(&mut income, id, "2024-01", "2800.00");
    assert_eq!(income.entry_count(id), 1);
}

#[test]
fn test_csd001_ac_5_3_month_after_job_end_is_accepted() {
    let mut income = Income::default();
    let id = job_with(&mut income, "01/03/2024", "31/01/2025");
    add(&mut income, id, "2025-03", "1500.00");
    assert_eq!(income.entry_count(id), 1);
}

#[test]
fn test_csd001_ac_5_4_month_later_than_current_is_rejected() {
    let mut income = Income::default();
    let id = job_with(&mut income, "01/03/2024", "");
    let error = income
        .add_paycheck(id, month("2026-11"), amounts("3000.00", "", ""), today())
        .unwrap_err();
    assert_eq!(error.code(), "PAYCHECK_MONTH_IN_FUTURE");
    assert_eq!(income.entry_count(id), 0);
}

#[test]
fn test_csd001_ac_5_5_current_month_is_accepted() {
    let mut income = Income::default();
    let id = job_with(&mut income, "01/03/2024", "");
    add(&mut income, id, "2026-10", "3000.00");
    assert_eq!(income.entry_count(id), 1);
}

#[test]
fn adding_a_paycheck_to_an_unknown_job_is_rejected() {
    let mut income = Income::default();
    let error = income
        .add_paycheck(
            JobId::generate(),
            month("2026-09"),
            amounts("1.00", "", ""),
            today(),
        )
        .unwrap_err();
    assert_eq!(error, DomainError::JobNotFound);
}

// ---- REQ-6: one paycheck per job per month ----

#[test]
fn test_csd001_ac_6_1_duplicate_month_rejected() {
    let mut income = Income::default();
    let id = job_with(&mut income, "01/03/2024", "");
    add(&mut income, id, "2026-09", "3000.00");
    let error = income
        .add_paycheck(id, month("2026-09"), amounts("3100.00", "", ""), today())
        .unwrap_err();
    assert_eq!(error.code(), "PAYCHECK_DUPLICATE_MONTH");
    let paychecks = income.paychecks(id).unwrap();
    assert_eq!(paychecks.len(), 1);
    assert_eq!(paychecks[0].amounts.gross_salary().to_string(), "3000.00");
}

#[test]
fn test_csd001_ac_6_2_same_month_on_another_job_is_allowed() {
    let mut income = Income::default();
    let first = job_with(&mut income, "01/03/2024", "");
    let second = job_with(&mut income, "01/01/2025", "");
    add(&mut income, first, "2026-09", "3000.00");
    add(&mut income, second, "2026-09", "1000.00");
    assert_eq!(income.entry_count(first), 1);
    assert_eq!(income.entry_count(second), 1);
}

// ---- REQ-7: amount validation ----

#[test]
fn test_csd001_ac_7_1_invalid_amounts_rejected() {
    let cases = [
        ("0.00", "0.00", "0.00", "PAYCHECK_GROSS_INVALID"),
        ("-100.00", "0.00", "0.00", "PAYCHECK_AMOUNT_NEGATIVE"),
        ("3000.00", "-1.00", "0.00", "PAYCHECK_AMOUNT_NEGATIVE"),
        ("3000.00", "0.00", "-5.00", "PAYCHECK_AMOUNT_NEGATIVE"),
        ("3000.999", "0.00", "0.00", "PAYCHECK_AMOUNT_PRECISION"),
        ("1000000.01", "0.00", "0.00", "PAYCHECK_AMOUNT_TOO_LARGE"),
    ];
    for (gross, holiday, bonus, code) in cases {
        let mut income = Income::default();
        let id = job_with(&mut income, "01/03/2024", "");
        let error = Amounts::parse(gross, holiday, bonus).unwrap_err();
        assert_eq!(error.code(), code, "{gross} {holiday} {bonus}");
        assert_eq!(income.paychecks(id).unwrap().len(), 0);
    }
}

#[test]
fn test_csd001_ac_7_2_boundary_amount_accepted() {
    let mut income = Income::default();
    let id = job_with(&mut income, "01/03/2024", "");
    add(&mut income, id, "2026-09", "1000000.00");
    assert_eq!(income.entry_count(id), 1);
}

// ---- REQ-8: edit and delete a paycheck ----

#[test]
fn test_csd001_ac_8_1_edit_amounts() {
    let mut income = Income::default();
    let id = job_with(&mut income, "01/03/2024", "");
    add(&mut income, id, "2026-05", "3000.00");
    let updated = income
        .update_paycheck(id, month("2026-05"), amounts("3000.00", "2400.00", "0.00"))
        .unwrap();
    assert_eq!(updated.amounts.holiday_pay().to_string(), "2400.00");
    assert_eq!(updated.amounts.total_gross().to_string(), "5400.00");
}

#[test]
fn test_csd001_ac_8_2_invalid_edit_rejected() {
    let mut income = Income::default();
    let id = job_with(&mut income, "01/03/2024", "");
    add(&mut income, id, "2026-05", "3000.00");
    let error = Amounts::parse("-1.00", "", "").unwrap_err();
    assert_eq!(error.code(), "PAYCHECK_AMOUNT_NEGATIVE");
    let kept = income.paychecks(id).unwrap();
    assert_eq!(kept[0].amounts.gross_salary().to_string(), "3000.00");
}

#[test]
fn test_csd001_ac_8_3_delete_paycheck() {
    let mut income = Income::default();
    let id = job_with(&mut income, "01/03/2024", "");
    add(&mut income, id, "2026-04", "3000.00");
    add(&mut income, id, "2026-05", "3000.00");
    income.delete_paycheck(id, month("2026-05")).unwrap();
    let months: Vec<String> = income
        .paychecks(id)
        .unwrap()
        .iter()
        .map(|p| p.month.to_string())
        .collect();
    assert_eq!(months, ["2026-04"]);
}

#[test]
fn test_csd001_ac_8_4_month_stays_fixed_on_edit() {
    let mut income = Income::default();
    let id = job_with(&mut income, "01/03/2024", "");
    add(&mut income, id, "2026-05", "3000.00");
    let updated = income
        .update_paycheck(id, month("2026-05"), amounts("3100.00", "", ""))
        .unwrap();
    assert_eq!(updated.month.to_string(), "2026-05");
    // The API has no way to move a paycheck: only the amounts are editable.
    assert_eq!(income.paychecks(id).unwrap().len(), 1);
}

#[test]
fn editing_or_deleting_a_missing_paycheck_is_rejected() {
    let mut income = Income::default();
    let id = job_with(&mut income, "01/03/2024", "");
    assert_eq!(
        income
            .update_paycheck(id, month("2026-05"), amounts("1.00", "", ""))
            .unwrap_err(),
        DomainError::PaycheckNotFound
    );
    assert_eq!(
        income.delete_paycheck(id, month("2026-05")).unwrap_err(),
        DomainError::PaycheckNotFound
    );
}

// ---- REQ-9: copy previous month ----

#[test]
fn test_csd001_ac_9_1_form_prefilled_from_latest_paycheck() {
    let mut income = Income::default();
    let id = job_with(&mut income, "01/03/2024", "");
    add(&mut income, id, "2026-07", "3000.00");
    income
        .add_paycheck(
            id,
            month("2026-08"),
            amounts("3100.00", "", "500.00"),
            today(),
        )
        .unwrap();
    let prefill = income.prefill_from_latest(id).unwrap();
    assert_eq!(prefill.gross_salary.unwrap().to_string(), "3100.00");
    assert_eq!(prefill.holiday_pay.to_string(), "0.00");
    assert_eq!(prefill.bonus.to_string(), "0.00");
    assert_eq!(income.entry_count(id), 2);
}

#[test]
fn test_csd001_ac_9_2_no_paycheck_to_copy() {
    let mut income = Income::default();
    let id = job_with(&mut income, "01/03/2024", "");
    let prefill = income.prefill_from_latest(id).unwrap();
    assert_eq!(prefill.gross_salary, None);
    assert_eq!(prefill.holiday_pay.to_string(), "0.00");
    assert_eq!(prefill.bonus.to_string(), "0.00");
}

// ---- REQ-10: total gross ----

#[test]
fn test_csd001_ac_10_1_total_is_the_sum() {
    let amounts = amounts("3000.00", "240.00", "500.00");
    assert_eq!(amounts.total_gross().to_string(), "3740.00");
}

// ---- REQ-11: yearly totals ----

#[test]
fn test_csd001_ac_11_1_totals_per_calendar_year() {
    let mut income = Income::default();
    let id = job_with(&mut income, "01/03/2024", "");
    add(&mut income, id, "2025-11", "3000.00");
    income
        .add_paycheck(
            id,
            month("2025-12"),
            amounts("3000.00", "", "1000.00"),
            today(),
        )
        .unwrap();
    income
        .add_paycheck(
            id,
            month("2026-05"),
            amounts("3000.00", "2400.00", ""),
            today(),
        )
        .unwrap();
    let totals = income.yearly_totals();
    assert_eq!(totals.len(), 2);
    let (y2026, y2025) = (&totals[0], &totals[1]);
    assert_eq!(y2026.year, 2026);
    assert_eq!(y2026.gross_salary.to_string(), "3000.00");
    assert_eq!(y2026.holiday_pay.to_string(), "2400.00");
    assert_eq!(y2026.bonus.to_string(), "0.00");
    assert_eq!(y2026.total_gross.to_string(), "5400.00");
    assert_eq!(y2026.entry_count, 1);
    assert_eq!(y2025.year, 2025);
    assert_eq!(y2025.gross_salary.to_string(), "6000.00");
    assert_eq!(y2025.holiday_pay.to_string(), "0.00");
    assert_eq!(y2025.bonus.to_string(), "1000.00");
    assert_eq!(y2025.total_gross.to_string(), "7000.00");
    assert_eq!(y2025.entry_count, 2);
}

#[test]
fn test_csd001_ac_11_2_no_paychecks_gives_no_totals() {
    let mut income = Income::default();
    job_with(&mut income, "01/03/2024", "");
    assert_eq!(income.yearly_totals().len(), 0);
}

#[test]
fn test_csd001_ac_11_3_totals_add_up_all_jobs() {
    let mut income = Income::default();
    let first = job_with(&mut income, "01/03/2024", "");
    let second = job_with(&mut income, "01/01/2020", "");
    add(&mut income, first, "2026-08", "3000.00");
    income
        .add_paycheck(
            first,
            month("2026-09"),
            amounts("3000.00", "", "500.00"),
            today(),
        )
        .unwrap();
    add(&mut income, second, "2026-09", "1000.00");

    let totals = income.yearly_totals();
    assert_eq!(totals.len(), 1);
    let year = &totals[0];
    assert_eq!(year.year, 2026);
    assert_eq!(year.gross_salary.to_string(), "7000.00");
    assert_eq!(year.holiday_pay.to_string(), "0.00");
    assert_eq!(year.bonus.to_string(), "500.00");
    assert_eq!(year.total_gross.to_string(), "7500.00");
    assert_eq!(year.entry_count, 3);
}

// ---- REQ-12: ordering ----

#[test]
fn test_csd001_ac_12_1_newest_first() {
    let mut income = Income::default();
    let old = job_with(&mut income, "01/01/2020", "");
    let recent = job_with(&mut income, "01/03/2024", "");
    add(&mut income, recent, "2026-07", "3000.00");
    add(&mut income, recent, "2026-09", "3000.00");
    let jobs: Vec<JobId> = income.jobs().iter().map(|job| job.id).collect();
    assert_eq!(jobs, [recent, old]);
    let months: Vec<String> = income
        .paychecks(recent)
        .unwrap()
        .iter()
        .map(|p| p.month.to_string())
        .collect();
    assert_eq!(months, ["2026-09", "2026-07"]);
}

#[test]
fn test_csd001_ac_12_2_same_month_is_ordered_by_job() {
    let mut income = Income::default();
    let old = job_with(&mut income, "01/01/2020", "");
    let recent = job_with(&mut income, "01/03/2024", "");
    // Added in the opposite order of the expected result.
    add(&mut income, old, "2026-09", "1000.00");
    add(&mut income, recent, "2026-09", "3000.00");
    add(&mut income, old, "2026-10", "1000.00");

    let order: Vec<(String, JobId)> = income
        .all_entries()
        .into_iter()
        .map(|(job, entry)| (entry.period(), job.id))
        .collect();
    assert_eq!(
        order,
        [
            ("2026-10".to_owned(), old),
            ("2026-09".to_owned(), recent),
            ("2026-09".to_owned(), old),
        ]
    );
}

#[test]
fn test_csd001_ac_12_3_yearly_totals_newest_year_first() {
    let mut income = Income::default();
    let id = job_with(&mut income, "01/03/2024", "");
    add(&mut income, id, "2024-05", "1000.00");
    add(&mut income, id, "2026-05", "1000.00");
    add(&mut income, id, "2025-05", "1000.00");
    let years: Vec<i32> = income.yearly_totals().iter().map(|t| t.year).collect();
    assert_eq!(years, [2026, 2025, 2024]);
}

// ---- REQ-13: empty states ----

#[test]
fn test_csd001_ac_13_1_no_jobs() {
    assert_eq!(Income::default().jobs().len(), 0);
}

#[test]
fn test_csd001_ac_13_2_job_without_paychecks_is_not_an_error() {
    let mut income = Income::default();
    let id = job_with(&mut income, "01/03/2024", "");
    assert_eq!(income.paychecks(id).unwrap().len(), 0);
    assert_eq!(income.all_entries().len(), 0);
    assert_eq!(income.yearly_totals().len(), 0);
}

// ---- REQ-23: employment view (data behind the paychecks table) ----

#[test]
fn test_csd001_ac_23_3_paychecks_of_all_jobs_come_with_their_job() {
    let mut income = Income::default();
    let recent = job_with(&mut income, "01/03/2024", "");
    let older = income
        .create_job(&input("Older Example Employer", "01/01/2020", ""))
        .unwrap()
        .id;
    add(&mut income, recent, "2026-09", "3000.00");
    add(&mut income, older, "2026-08", "1000.00");

    let rows: Vec<(&str, String)> = income
        .all_entries()
        .into_iter()
        .map(|(job, entry)| (job.employer.as_str(), entry.period()))
        .collect();
    assert_eq!(
        rows,
        [
            ("Example Employer", "2026-09".to_owned()),
            ("Older Example Employer", "2026-08".to_owned()),
        ]
    );
}

// ---- CSD-004: income dashboard ----

/// The paychecks of CSD-004 AC-2.1: two jobs, today 25/10/2026.
fn dashboard_income() -> Income {
    let mut income = Income::default();
    let recent = job_with(&mut income, "01/03/2024", "");
    let older = income
        .create_job(&input("Older Example Employer", "01/01/2020", ""))
        .unwrap()
        .id;
    add(&mut income, recent, "2026-08", "3000.00");
    income
        .add_paycheck(
            recent,
            month("2026-09"),
            amounts("3000.00", "", "500.00"),
            today(),
        )
        .unwrap();
    add(&mut income, recent, "2026-10", "3000.00");
    add(&mut income, older, "2026-09", "1000.00");
    income
}

#[test]
fn test_csd004_ac_2_1_year_to_date_over_all_jobs() {
    let overview = dashboard_income().overview(today());
    assert_eq!(overview.year, 2026);
    let totals = overview.year_to_date.unwrap();
    assert_eq!(totals.gross_salary.to_string(), "10000.00");
    assert_eq!(totals.holiday_pay.to_string(), "0.00");
    assert_eq!(totals.bonus.to_string(), "500.00");
    assert_eq!(totals.total_gross.to_string(), "10500.00");
    assert_eq!(totals.entry_count, 4);
}

#[test]
fn test_csd004_ac_2_2_other_years_are_not_counted() {
    let mut income = Income::default();
    let id = job_with(&mut income, "01/03/2024", "");
    add(&mut income, id, "2025-12", "3000.00");
    add(&mut income, id, "2026-09", "1000.00");
    let totals = income.overview(today()).year_to_date.unwrap();
    assert_eq!(totals.gross_salary.to_string(), "1000.00");
    assert_eq!(totals.entry_count, 1);
}

#[test]
fn test_csd004_ac_2_3_months_after_the_current_month_are_not_counted() {
    let mut income = Income::default();
    let id = job_with(&mut income, "01/03/2024", "");
    add(&mut income, id, "2026-10", "3000.00");
    // Only a project file can hold a later month, so allow it here.
    income
        .add_paycheck(
            id,
            month("2026-12"),
            amounts("3000.00", "", ""),
            month("2026-12"),
        )
        .unwrap();
    let totals = income.overview(today()).year_to_date.unwrap();
    assert_eq!(totals.gross_salary.to_string(), "3000.00");
    assert_eq!(totals.entry_count, 1);
}

#[test]
fn test_csd004_ac_3_1_last_month_is_the_previous_calendar_month() {
    let overview = dashboard_income().overview(today());
    assert_eq!(overview.last_month.unwrap().to_string(), "2026-09");
    let totals = overview.last_month_totals.unwrap();
    assert_eq!(totals.gross_salary.to_string(), "4000.00");
    assert_eq!(totals.holiday_pay.to_string(), "0.00");
    assert_eq!(totals.bonus.to_string(), "500.00");
    assert_eq!(totals.total_gross.to_string(), "4500.00");
    assert_eq!(totals.entry_count, 2);
}

#[test]
fn test_csd004_ac_3_2_january_looks_back_to_december() {
    let mut income = Income::default();
    let id = job_with(&mut income, "01/03/2024", "");
    income
        .add_paycheck(
            id,
            month("2026-12"),
            amounts("3000.00", "", ""),
            month("2027-01"),
        )
        .unwrap();
    let overview = income.overview(month("2027-01"));
    assert_eq!(overview.last_month.unwrap().to_string(), "2026-12");
    let totals = overview.last_month_totals.unwrap();
    assert_eq!(totals.total_gross.to_string(), "3000.00");
    assert_eq!(totals.entry_count, 1);
}

#[test]
fn test_csd004_ac_3_3_the_current_month_is_not_last_month() {
    let mut income = Income::default();
    let id = job_with(&mut income, "01/03/2024", "");
    add(&mut income, id, "2026-10", "3000.00");
    let overview = income.overview(today());
    assert_eq!(overview.last_month.unwrap().to_string(), "2026-09");
    assert_eq!(overview.last_month_totals, None);
}

#[test]
fn test_csd004_ac_4_1_nothing_earned_this_year_yet_has_no_totals() {
    let mut income = Income::default();
    let id = job_with(&mut income, "01/03/2024", "");
    income
        .add_paycheck(
            id,
            month("2026-12"),
            amounts("3000.00", "", ""),
            month("2027-01"),
        )
        .unwrap();
    let overview = income.overview(month("2027-01"));
    assert_eq!(overview.year, 2027);
    assert_eq!(overview.year_to_date, None);
}

#[test]
fn test_csd004_ac_4_2_nothing_earned_last_month_has_no_totals() {
    let mut income = Income::default();
    let id = job_with(&mut income, "01/03/2024", "");
    add(&mut income, id, "2026-10", "3000.00");
    let overview = income.overview(today());
    assert_eq!(overview.last_month_totals, None);
    assert!(overview.year_to_date.is_some());
}

#[test]
fn test_csd004_ac_5_1_has_jobs_tells_an_empty_project_apart() {
    let mut income = Income::default();
    assert!(!income.has_jobs());
    job_with(&mut income, "01/03/2024", "");
    assert!(income.has_jobs());
    // A job without paychecks has no totals, and no zero amounts either.
    let overview = income.overview(today());
    assert_eq!(overview.year_to_date, None);
    assert_eq!(overview.last_month_totals, None);
}

#[test]
fn year_month_previous_wraps_and_stops_at_the_first_year() {
    assert_eq!(month("2026-10").previous().unwrap().to_string(), "2026-09");
    assert_eq!(month("2027-01").previous().unwrap().to_string(), "2026-12");
    assert_eq!(month("1900-01").previous(), None);
}

// ---- REQ-22: date format ----

#[test]
fn test_csd001_ac_22_1_typed_dates_become_calendar_dates() {
    let mut income = Income::default();
    let job = income
        .create_job(&input("Example Employer", "01/03/2024", "31/01/2025"))
        .unwrap();
    // The domain stores real dates; commands return them as ISO 8601 and the
    // frontend shows them as dd/mm/yyyy (manual checklist for the display).
    assert_eq!(job.start_date.to_string(), "2024-03-01");
    assert_eq!(job.end_date.unwrap().to_string(), "2025-01-31");
    let json = serde_json::to_value(&job).unwrap();
    assert_eq!(json["start_date"], "2024-03-01");
    assert_eq!(json["end_date"], "2025-01-31");
}

#[test]
fn test_csd001_ac_22_2_other_date_formats_are_rejected() {
    let cases = [
        ("2024-03-01", "year first"),
        ("1/3/2024", "no two-digit day and month"),
        ("1/03/2024", "no two-digit day"),
        ("01/3/2024", "no two-digit month"),
        ("03-01-2024", "wrong separator"),
        ("31/02/2024", "not a calendar date"),
        ("29/02/2023", "not a leap year"),
        ("01/13/2024", "month 13"),
        ("00/03/2024", "day 0"),
        ("01/03/24", "two-digit year"),
        ("01/03/20244", "five-digit year"),
        ("01.03.2024", "dots"),
        ("1 March 2024", "words"),
        ("01/03/2024 10:00", "time attached"),
    ];
    for (start, reason) in cases {
        let mut income = Income::default();
        let error = income
            .create_job(&input("Example Employer", start, ""))
            .unwrap_err();
        assert_eq!(error.code(), "JOB_DATE_INVALID", "{start:?}: {reason}");
        assert_eq!(income.jobs().len(), 0, "{start:?}: {reason}");
    }
}

#[test]
fn test_csd001_ac_22_2_end_date_uses_the_same_format() {
    let mut income = Income::default();
    let error = income
        .create_job(&input("Example Employer", "01/03/2024", "2025-01-31"))
        .unwrap_err();
    assert_eq!(error, DomainError::JobDateInvalid);
    let id = job_with(&mut income, "01/03/2024", "");
    let error = income
        .update_job(id, &input("Example Employer", "01/03/2024", "31-01-2025"))
        .unwrap_err();
    assert_eq!(error, DomainError::JobDateInvalid);
}

#[test]
fn dates_are_accepted_with_surrounding_whitespace_and_on_leap_days() {
    let mut income = Income::default();
    let job = income
        .create_job(&input("Example Employer", " 29/02/2024 ", ""))
        .unwrap();
    assert_eq!(job.start_date.to_string(), "2024-02-29");
}

#[test]
fn the_date_error_message_names_the_expected_format() {
    assert_eq!(
        DomainError::JobDateInvalid.to_string(),
        "Enter dates as dd/mm/yyyy."
    );
}

#[test]
fn test_csd001_ac_22_3_months_stay_year_month() {
    let mut income = Income::default();
    let id = job_with(&mut income, "01/03/2024", "");
    let paycheck = add(&mut income, id, "2026-09", "3000.00");
    assert_eq!(paycheck.month.to_string(), "2026-09");
    let json = serde_json::to_value(&paycheck).unwrap();
    assert_eq!(json["month"], "2026-09");
    for bad in ["09/2026", "01/09/2026", "09-2026"] {
        assert_eq!(
            YearMonth::parse(bad).unwrap_err(),
            DomainError::PaycheckMonthInvalid,
            "{bad:?}"
        );
    }
}

// ---- annual statements (CSD-001 0.9) ----

fn year(text: &str) -> Year {
    Year::parse(text).unwrap()
}

fn yearly_job(income: &mut Income) -> JobId {
    income
        .create_job(&JobInput {
            pay_period: Some("Yearly".to_owned()),
            ..input("Example Employer", "01/03/2024", "")
        })
        .unwrap()
        .id
}

fn statement(income: &mut Income, job: JobId, y: &str, gross: &str) -> AnnualStatement {
    income
        .add_statement(job, year(y), amounts(gross, "", ""), today())
        .unwrap()
}

#[test]
fn test_csd001_ac_1_5_a_job_can_be_yearly() {
    let mut income = Income::default();
    let id = yearly_job(&mut income);
    assert_eq!(income.job(id).unwrap().pay_period, PayPeriod::Yearly);
}

#[test]
fn test_csd001_ac_1_1_a_job_is_monthly_by_default() {
    let mut income = Income::default();
    let id = job_with(&mut income, "01/03/2024", "");
    assert_eq!(income.job(id).unwrap().pay_period, PayPeriod::Monthly);
    // Blank text means the default as well.
    let blank = income
        .create_job(&JobInput {
            pay_period: Some("  ".to_owned()),
            ..input("Example Employer", "01/03/2024", "")
        })
        .unwrap();
    assert_eq!(blank.pay_period, PayPeriod::Monthly);
}

#[test]
fn test_csd001_ac_1_6_unsupported_pay_periods_are_rejected() {
    for text in ["Weekly", "monthly!", "Year", "12"] {
        let mut income = Income::default();
        let error = income
            .create_job(&JobInput {
                pay_period: Some(text.to_owned()),
                ..input("Example Employer", "01/03/2024", "")
            })
            .unwrap_err();
        assert_eq!(error.code(), "JOB_PERIOD_UNSUPPORTED", "{text:?}");
        assert_eq!(income.jobs().len(), 0);
    }
}

#[test]
fn pay_periods_are_parsed_case_insensitively() {
    assert_eq!(PayPeriod::parse(" yearly "), Some(PayPeriod::Yearly));
    assert_eq!(PayPeriod::parse("MONTHLY"), Some(PayPeriod::Monthly));
    assert_eq!(PayPeriod::parse(""), None);
}

#[test]
fn test_csd001_ac_24_1_the_pay_period_is_locked_once_entries_exist() {
    let mut income = Income::default();
    let monthly = job_with(&mut income, "01/03/2024", "");
    add(&mut income, monthly, "2026-09", "3000.00");
    let error = income
        .update_job(
            monthly,
            &JobInput {
                pay_period: Some("Yearly".to_owned()),
                ..input("Example Employer", "01/03/2024", "")
            },
        )
        .unwrap_err();
    assert_eq!(error.code(), "JOB_PERIOD_LOCKED");
    assert_eq!(income.job(monthly).unwrap().pay_period, PayPeriod::Monthly);

    let yearly = yearly_job(&mut income);
    statement(&mut income, yearly, "2025", "40000.00");
    let error = income
        .update_job(
            yearly,
            &JobInput {
                pay_period: Some("Monthly".to_owned()),
                ..input("Example Employer", "01/03/2024", "")
            },
        )
        .unwrap_err();
    assert_eq!(error, DomainError::JobPeriodLocked);
}

#[test]
fn test_csd001_ac_24_2_the_pay_period_can_change_without_entries() {
    let mut income = Income::default();
    let id = job_with(&mut income, "01/03/2024", "");
    let updated = income
        .update_job(
            id,
            &JobInput {
                pay_period: Some("Yearly".to_owned()),
                ..input("Example Employer", "01/03/2024", "")
            },
        )
        .unwrap();
    assert_eq!(updated.pay_period, PayPeriod::Yearly);
}

#[test]
fn test_csd001_ac_24_3_keeping_or_omitting_the_pay_period_is_allowed_with_entries() {
    let mut income = Income::default();
    let id = yearly_job(&mut income);
    statement(&mut income, id, "2025", "40000.00");
    // Same value, and no value at all ("unchanged"), both keep Yearly.
    for period in [Some("Yearly".to_owned()), None] {
        let updated = income
            .update_job(
                id,
                &JobInput {
                    pay_period: period,
                    ..input("Renamed Employer", "01/03/2024", "")
                },
            )
            .unwrap();
        assert_eq!(updated.employer, "Renamed Employer");
        assert_eq!(updated.pay_period, PayPeriod::Yearly);
    }
}

#[test]
fn test_csd001_ac_25_1_an_annual_statement_has_defaults() {
    let mut income = Income::default();
    let id = yearly_job(&mut income);
    let statement = statement(&mut income, id, "2025", "40000.00");
    assert_eq!(statement.amounts.gross_salary().to_string(), "40000.00");
    assert_eq!(statement.amounts.holiday_pay().to_string(), "0.00");
    assert_eq!(statement.amounts.bonus().to_string(), "0.00");
    assert_eq!(statement.amounts.total_gross().to_string(), "40000.00");
}

#[test]
fn test_csd001_ac_25_2_a_year_before_the_job_start_is_accepted() {
    let mut income = Income::default();
    let id = yearly_job(&mut income);
    statement(&mut income, id, "2022", "38000.00");
    assert_eq!(income.entry_count(id), 1);
}

#[test]
fn test_csd001_ac_25_3_a_year_later_than_the_current_year_is_rejected() {
    let mut income = Income::default();
    let id = yearly_job(&mut income);
    let error = income
        .add_statement(id, year("2027"), amounts("40000.00", "", ""), today())
        .unwrap_err();
    assert_eq!(error.code(), "STATEMENT_YEAR_IN_FUTURE");
    assert_eq!(income.entry_count(id), 0);
}

#[test]
fn test_csd001_ac_25_4_the_current_year_is_accepted() {
    let mut income = Income::default();
    let id = yearly_job(&mut income);
    statement(&mut income, id, "2026", "40000.00");
    assert_eq!(income.entry_count(id), 1);
}

#[test]
fn test_csd001_ac_25_5_an_invalid_year_is_rejected() {
    for text in [
        "26", "2026-05", "abcd", "1899", "", " ", "20266", "-202", "2026 ",
    ] {
        let result = Year::parse(text);
        if text.trim() == "2026" {
            // Surrounding whitespace is trimmed like other typed input.
            assert_eq!(result.unwrap().to_string(), "2026");
        } else {
            assert_eq!(
                result.unwrap_err().code(),
                "STATEMENT_YEAR_INVALID",
                "{text:?}"
            );
        }
    }
    assert_eq!(year("1900").to_string(), "1900");
    assert_eq!(year("9999").to_string(), "9999");
}

#[test]
fn test_csd001_ac_26_1_a_duplicate_year_is_rejected() {
    let mut income = Income::default();
    let id = yearly_job(&mut income);
    statement(&mut income, id, "2025", "40000.00");
    let error = income
        .add_statement(id, year("2025"), amounts("41000.00", "", ""), today())
        .unwrap_err();
    assert_eq!(error.code(), "STATEMENT_DUPLICATE_YEAR");
    let kept = income.statements(id).unwrap();
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].amounts.gross_salary().to_string(), "40000.00");
}

#[test]
fn test_csd001_ac_26_2_the_same_year_on_another_job_is_allowed() {
    let mut income = Income::default();
    let first = yearly_job(&mut income);
    let second = yearly_job(&mut income);
    statement(&mut income, first, "2025", "40000.00");
    statement(&mut income, second, "2025", "30000.00");
    assert_eq!(income.entry_count(first), 1);
    assert_eq!(income.entry_count(second), 1);
}

#[test]
fn test_csd001_ac_27_1_a_paycheck_for_a_yearly_job_is_rejected() {
    let mut income = Income::default();
    let id = yearly_job(&mut income);
    let error = income
        .add_paycheck(id, month("2026-09"), amounts("3000.00", "", ""), today())
        .unwrap_err();
    assert_eq!(error.code(), "JOB_PERIOD_MISMATCH");
    assert_eq!(income.entry_count(id), 0);
}

#[test]
fn test_csd001_ac_27_2_a_statement_for_a_monthly_job_is_rejected() {
    let mut income = Income::default();
    let id = job_with(&mut income, "01/03/2024", "");
    let error = income
        .add_statement(id, year("2025"), amounts("40000.00", "", ""), today())
        .unwrap_err();
    assert_eq!(error.code(), "JOB_PERIOD_MISMATCH");
    assert_eq!(income.entry_count(id), 0);
}

#[test]
fn adding_a_statement_to_an_unknown_job_is_rejected() {
    let mut income = Income::default();
    let error = income
        .add_statement(
            JobId::generate(),
            year("2025"),
            amounts("1.00", "", ""),
            today(),
        )
        .unwrap_err();
    assert_eq!(error, DomainError::JobNotFound);
}

#[test]
fn test_csd001_ac_8_5_a_statement_can_be_edited_and_deleted() {
    let mut income = Income::default();
    let id = yearly_job(&mut income);
    statement(&mut income, id, "2025", "40000.00");
    let updated = income
        .update_statement(id, year("2025"), amounts("40000.00", "", "3000.00"))
        .unwrap();
    assert_eq!(updated.amounts.bonus().to_string(), "3000.00");
    assert_eq!(updated.amounts.total_gross().to_string(), "43000.00");
    income.delete_statement(id, year("2025")).unwrap();
    assert_eq!(income.statements(id).unwrap().len(), 0);
    assert_eq!(
        income.delete_statement(id, year("2025")).unwrap_err(),
        DomainError::StatementNotFound
    );
    assert_eq!(
        income
            .update_statement(id, year("2025"), amounts("1.00", "", ""))
            .unwrap_err(),
        DomainError::StatementNotFound
    );
}

#[test]
fn test_csd001_ac_8_6_the_year_of_a_statement_is_fixed() {
    let mut income = Income::default();
    let id = yearly_job(&mut income);
    statement(&mut income, id, "2025", "40000.00");
    let updated = income
        .update_statement(id, year("2025"), amounts("41000.00", "", ""))
        .unwrap();
    assert_eq!(updated.year.to_string(), "2025");
    // The API has no way to move a statement: only the amounts are editable.
    assert_eq!(income.statements(id).unwrap().len(), 1);
}

#[test]
fn test_csd001_ac_8_7_an_invalid_statement_edit_is_rejected() {
    let mut income = Income::default();
    let id = yearly_job(&mut income);
    statement(&mut income, id, "2025", "40000.00");
    let error = Amounts::parse("-1.00", "", "").unwrap_err();
    assert_eq!(error.code(), "PAYCHECK_AMOUNT_NEGATIVE");
    let kept = income.statements(id).unwrap();
    assert_eq!(kept[0].amounts.gross_salary().to_string(), "40000.00");
}

#[test]
fn test_csd001_ac_9_4_copy_previous_year_uses_the_latest_statement() {
    let mut income = Income::default();
    let id = yearly_job(&mut income);
    statement(&mut income, id, "2024", "40000.00");
    income
        .add_statement(
            id,
            year("2025"),
            amounts("42000.00", "", "3000.00"),
            today(),
        )
        .unwrap();
    let prefill = income.prefill_from_latest(id).unwrap();
    assert_eq!(prefill.gross_salary.unwrap().to_string(), "42000.00");
    assert_eq!(prefill.holiday_pay.to_string(), "0.00");
    assert_eq!(prefill.bonus.to_string(), "0.00");
    assert_eq!(income.entry_count(id), 2);
}

#[test]
fn test_csd001_ac_9_5_no_statement_to_copy() {
    let mut income = Income::default();
    let id = yearly_job(&mut income);
    let prefill = income.prefill_from_latest(id).unwrap();
    assert_eq!(prefill.gross_salary, None);
    assert_eq!(prefill.holiday_pay.to_string(), "0.00");
    assert_eq!(prefill.bonus.to_string(), "0.00");
}

#[test]
fn test_csd001_ac_10_2_total_of_an_annual_statement() {
    let amounts = amounts("40000.00", "2000.00", "3000.00");
    assert_eq!(amounts.total_gross().to_string(), "45000.00");
}

#[test]
fn test_csd001_ac_11_5_statements_count_in_their_year_next_to_paychecks() {
    let mut income = Income::default();
    let yearly = yearly_job(&mut income);
    let monthly = job_with(&mut income, "01/01/2020", "");
    income
        .add_statement(
            yearly,
            year("2025"),
            amounts("40000.00", "", "3000.00"),
            today(),
        )
        .unwrap();
    add(&mut income, monthly, "2025-12", "3000.00");

    let totals = income.yearly_totals();
    assert_eq!(totals.len(), 1);
    assert_eq!(totals[0].year, 2025);
    assert_eq!(totals[0].gross_salary.to_string(), "43000.00");
    assert_eq!(totals[0].holiday_pay.to_string(), "0.00");
    assert_eq!(totals[0].bonus.to_string(), "3000.00");
    assert_eq!(totals[0].total_gross.to_string(), "46000.00");
    assert_eq!(totals[0].entry_count, 2);
}

#[test]
fn test_csd001_ac_11_6_a_statement_does_not_count_in_other_years() {
    let mut income = Income::default();
    let id = yearly_job(&mut income);
    statement(&mut income, id, "2025", "40000.00");
    let years: Vec<i32> = income.yearly_totals().iter().map(|t| t.year).collect();
    assert_eq!(years, [2025]);
}

#[test]
fn test_csd001_ac_12_4_a_statement_sorts_as_december() {
    let mut income = Income::default();
    let yearly = yearly_job(&mut income);
    let monthly = job_with(&mut income, "01/01/2020", "");
    add(&mut income, monthly, "2025-12", "3000.00");
    add(&mut income, monthly, "2026-10", "3000.00");
    statement(&mut income, yearly, "2026", "40000.00");
    let periods: Vec<String> = income
        .all_entries()
        .into_iter()
        .map(|(_, entry)| entry.period())
        .collect();
    assert_eq!(periods, ["2026", "2026-10", "2025-12"]);
}

#[test]
fn test_csd001_ac_12_5_the_same_period_is_ordered_by_job() {
    let mut income = Income::default();
    // "Example Employer" starts later, so it comes first in job order.
    let monthly = job_with(&mut income, "01/03/2024", "");
    let yearly = income
        .create_job(&JobInput {
            pay_period: Some("Yearly".to_owned()),
            ..input("Older Example Employer", "01/01/2020", "")
        })
        .unwrap()
        .id;
    // 2026-12 is later than today, so allow it for this ordering test.
    income
        .add_paycheck(
            monthly,
            month("2026-12"),
            amounts("3000.00", "", ""),
            month("2026-12"),
        )
        .unwrap();
    statement(&mut income, yearly, "2026", "40000.00");
    let rows: Vec<(String, bool)> = income
        .all_entries()
        .into_iter()
        .map(|(job, entry)| (job.employer.clone(), entry.is_statement()))
        .collect();
    assert_eq!(
        rows,
        [
            ("Example Employer".to_owned(), false),
            ("Older Example Employer".to_owned(), true),
        ]
    );
}

#[test]
fn test_csd001_ac_23_3_entries_of_both_kinds_come_with_job_and_period() {
    let mut income = Income::default();
    let monthly = job_with(&mut income, "01/03/2024", "");
    let yearly = income
        .create_job(&JobInput {
            pay_period: Some("Yearly".to_owned()),
            ..input("Older Example Employer", "01/01/2020", "")
        })
        .unwrap()
        .id;
    add(&mut income, monthly, "2026-09", "3000.00");
    statement(&mut income, yearly, "2025", "40000.00");
    let rows: Vec<(&str, String, &str)> = income
        .all_entries()
        .into_iter()
        .map(|(job, entry)| {
            (
                job.employer.as_str(),
                entry.period(),
                if entry.is_statement() {
                    "statement"
                } else {
                    "paycheck"
                },
            )
        })
        .collect();
    assert_eq!(
        rows,
        [
            ("Example Employer", "2026-09".to_owned(), "paycheck"),
            ("Older Example Employer", "2025".to_owned(), "statement"),
        ]
    );
}

#[test]
fn test_csd001_ac_4_1_deleting_a_job_removes_its_statements() {
    let mut income = Income::default();
    let id = yearly_job(&mut income);
    statement(&mut income, id, "2024", "40000.00");
    statement(&mut income, id, "2025", "42000.00");
    income.delete_job(id).unwrap();
    assert_eq!(income.all_entries().len(), 0);
    assert_eq!(income.yearly_totals().len(), 0);
}

// ---- CSD-004 with annual statements ----

#[test]
fn test_csd004_ac_2_4_an_annual_statement_counts_in_full_this_year() {
    let mut income = Income::default();
    let yearly = yearly_job(&mut income);
    let monthly = job_with(&mut income, "01/01/2020", "");
    income
        .add_statement(
            yearly,
            year("2026"),
            amounts("40000.00", "", "3000.00"),
            today(),
        )
        .unwrap();
    add(&mut income, monthly, "2026-09", "3000.00");
    let totals = income.overview(today()).year_to_date.unwrap();
    assert_eq!(totals.gross_salary.to_string(), "43000.00");
    assert_eq!(totals.holiday_pay.to_string(), "0.00");
    assert_eq!(totals.bonus.to_string(), "3000.00");
    assert_eq!(totals.total_gross.to_string(), "46000.00");
    assert_eq!(totals.entry_count, 2);
}

#[test]
fn test_csd004_ac_2_5_statements_of_other_years_are_not_counted() {
    let mut income = Income::default();
    let id = yearly_job(&mut income);
    statement(&mut income, id, "2025", "40000.00");
    assert_eq!(income.overview(today()).year_to_date, None);
}

#[test]
fn test_csd004_ac_3_4_annual_statements_are_not_last_month() {
    let mut income = Income::default();
    let id = yearly_job(&mut income);
    statement(&mut income, id, "2026", "40000.00");
    let overview = income.overview(today());
    assert_eq!(overview.last_month.unwrap().to_string(), "2026-09");
    assert_eq!(overview.last_month_totals, None);
    // The statement is still part of this year.
    assert!(overview.year_to_date.is_some());
}

// ---- data from outside (a project file) ----

#[test]
fn validate_rejects_entries_that_do_not_match_the_pay_period() {
    let mut income = Income::default();
    let monthly = job_with(&mut income, "01/03/2024", "");
    let yearly = yearly_job(&mut income);
    add(&mut income, monthly, "2026-09", "3000.00");
    statement(&mut income, yearly, "2025", "40000.00");
    assert!(income.validate().is_ok());

    // A paycheck of the yearly job, and a statement of the monthly job.
    let mut wrong_paycheck = income.clone();
    wrong_paycheck.paychecks[0].job_id = yearly;
    assert_eq!(
        wrong_paycheck.validate().unwrap_err(),
        DomainError::DataInvalid
    );
    let mut wrong_statement = income.clone();
    wrong_statement.statements[0].job_id = monthly;
    assert_eq!(
        wrong_statement.validate().unwrap_err(),
        DomainError::DataInvalid
    );

    let mut duplicate = income.clone();
    duplicate.statements.push(duplicate.statements[0].clone());
    assert_eq!(duplicate.validate().unwrap_err(), DomainError::DataInvalid);

    let mut orphan = income.clone();
    orphan.statements[0].job_id = JobId::generate();
    assert_eq!(orphan.validate().unwrap_err(), DomainError::DataInvalid);
}

#[test]
fn year_orders_and_prints_with_four_digits() {
    assert!(year("2025") < year("2026"));
    assert_eq!(year("1900").to_string(), "1900");
    assert_eq!(Year::new(1899), None);
    assert_eq!(Year::new(10000), None);
    let json = serde_json::to_string(&year("2025")).unwrap();
    assert_eq!(json, "\"2025\"");
    assert_eq!(serde_json::from_str::<Year>(&json).unwrap(), year("2025"));
    assert!(serde_json::from_str::<Year>("\"25\"").is_err());
}

// ---- supporting types ----

#[test]
fn year_month_round_trips_and_orders() {
    assert_eq!(month("2026-09").to_string(), "2026-09");
    assert!(month("2025-12") < month("2026-01"));
    for bad in [
        "2026-9", "26-09", "2026-13", "2026-00", "2026/09", "", "1899-12",
    ] {
        assert_eq!(
            YearMonth::parse(bad).unwrap_err(),
            DomainError::PaycheckMonthInvalid,
            "{bad:?}"
        );
    }
}

#[test]
fn validate_rejects_inconsistent_data() {
    let mut income = Income::default();
    let id = job_with(&mut income, "01/03/2024", "");
    add(&mut income, id, "2026-09", "3000.00");
    assert!(income.validate().is_ok());

    let mut orphan = income.clone();
    orphan.paychecks[0].job_id = JobId::generate();
    assert_eq!(orphan.validate().unwrap_err(), DomainError::DataInvalid);

    let mut duplicate = income.clone();
    duplicate.paychecks.push(duplicate.paychecks[0].clone());
    assert_eq!(duplicate.validate().unwrap_err(), DomainError::DataInvalid);

    let mut blank = income.clone();
    blank.jobs[0].employer = "  ".into();
    assert_eq!(blank.validate().unwrap_err(), DomainError::DataInvalid);
}

// ---- CSD-006: income insights ----

fn yearly_job_named(income: &mut Income, employer: &str) -> JobId {
    income
        .create_job(&JobInput {
            pay_period: Some("Yearly".to_owned()),
            ..input(employer, "01/01/2020", "")
        })
        .unwrap()
        .id
}

fn add_statement_of(income: &mut Income, job: JobId, y: &str, parts: [&str; 3]) {
    income
        .add_statement(job, year(y), amounts(parts[0], parts[1], parts[2]), today())
        .unwrap();
}

fn years_of(series: &[IncomeInsightsYear]) -> Vec<i32> {
    series.iter().map(|y| y.year).collect()
}

fn growth_of(series: &[IncomeInsightsYear], y: i32) -> Option<String> {
    series
        .iter()
        .find(|row| row.year == y)
        .unwrap()
        .growth_percent
        .map(|growth| growth.to_string())
}

#[test]
fn test_csd006_ac_2_1_series_runs_from_first_entry_to_current_year() {
    let mut income = Income::default();
    let id = job_with(&mut income, "01/01/2024", "");
    add(&mut income, id, "2024-03", "3000.00");
    add(&mut income, id, "2025-05", "3000.00");
    add(&mut income, id, "2026-09", "3000.00");
    assert_eq!(years_of(&income.insights(today())), [2024, 2025, 2026]);
}

#[test]
fn test_csd006_ac_2_2_two_jobs_are_summed() {
    let mut income = Income::default();
    let employer = yearly_job_named(&mut income, "Example Employer");
    let agency = yearly_job_named(&mut income, "Example Agency");
    add_statement_of(
        &mut income,
        employer,
        "2025",
        ["30000.00", "2400.00", "0.00"],
    );
    add_statement_of(&mut income, agency, "2025", ["5000.00", "0.00", "1000.00"]);
    let series = income.insights(today());
    let row = series.iter().find(|row| row.year == 2025).unwrap();
    assert_eq!(row.gross_salary.to_string(), "35000.00");
    assert_eq!(row.holiday_pay.to_string(), "2400.00");
    assert_eq!(row.bonus.to_string(), "1000.00");
    assert_eq!(row.total_gross.to_string(), "38400.00");
    assert_eq!(row.entry_count, 2);
}

#[test]
fn test_csd006_ac_2_3_a_single_year() {
    let mut income = Income::default();
    let id = job_with(&mut income, "01/03/2024", "");
    add(&mut income, id, "2026-09", "3000.00");
    assert_eq!(years_of(&income.insights(today())), [2026]);
}

#[test]
fn test_csd006_ac_3_1_gap_year_is_zero() {
    let mut income = Income::default();
    let id = yearly_job_named(&mut income, "Example Employer");
    add_statement_of(&mut income, id, "2022", ["30000.00", "", ""]);
    add_statement_of(&mut income, id, "2024", ["31000.00", "", ""]);
    let series = income.insights(today());
    assert_eq!(years_of(&series), [2022, 2023, 2024, 2025, 2026]);
    let gap = &series[1];
    assert_eq!(gap.total_gross.to_string(), "0.00");
    assert_eq!(gap.gross_salary.to_string(), "0.00");
    assert_eq!(gap.entry_count, 0);
}

#[test]
fn test_csd006_ac_4_1_only_the_current_year_is_partial() {
    let mut income = Income::default();
    let id = job_with(&mut income, "01/01/2024", "");
    add(&mut income, id, "2025-05", "3000.00");
    add(&mut income, id, "2026-09", "3000.00");
    let series = income.insights(today());
    assert!(!series[0].is_partial);
    assert!(series[1].is_partial);
}

#[test]
fn test_csd006_ac_5_1_growth_per_year() {
    for (previous, current, growth) in [
        ("32400.00", "34480.00", "6.4"),
        ("40000.00", "39000.00", "-2.5"),
        // Banker's rounding: 0.45 goes to the even 0.4, 0.55 to 0.6.
        ("20000.00", "20090.00", "0.4"),
        ("20000.00", "20110.00", "0.6"),
        ("20000.00", "20000.00", "0.0"),
        // A decrease that rounds to zero shows no sign.
        ("100000.00", "99960.00", "0.0"),
    ] {
        let mut income = Income::default();
        let id = yearly_job_named(&mut income, "Example Employer");
        add_statement_of(&mut income, id, "2023", [previous, "", ""]);
        add_statement_of(&mut income, id, "2024", [current, "", ""]);
        let series = income.insights(today());
        assert_eq!(
            growth_of(&series, 2024).as_deref(),
            Some(growth),
            "{previous} -> {current}"
        );
    }
}

#[test]
fn test_csd006_ac_6_1_first_year_has_no_growth() {
    let mut income = Income::default();
    let id = yearly_job_named(&mut income, "Example Employer");
    add_statement_of(&mut income, id, "2024", ["31000.00", "", ""]);
    assert_eq!(growth_of(&income.insights(today()), 2024), None);
}

#[test]
fn test_csd006_ac_6_2_current_year_has_no_growth() {
    let mut income = Income::default();
    let id = job_with(&mut income, "01/01/2025", "");
    add(&mut income, id, "2025-05", "34480.00");
    add(&mut income, id, "2026-05", "28000.00");
    assert_eq!(growth_of(&income.insights(today()), 2026), None);
}

#[test]
fn test_csd006_ac_6_3_year_after_a_gap_has_no_growth() {
    let mut income = Income::default();
    let id = yearly_job_named(&mut income, "Example Employer");
    add_statement_of(&mut income, id, "2022", ["30000.00", "", ""]);
    add_statement_of(&mut income, id, "2024", ["31000.00", "", ""]);
    let series = income.insights(today());
    assert_eq!(growth_of(&series, 2023), None);
    assert_eq!(growth_of(&series, 2024), None);
}

#[test]
fn test_csd006_ac_10_1_no_entries_gives_an_empty_series() {
    let mut income = Income::default();
    assert_eq!(income.insights(today()).len(), 0);
    job_with(&mut income, "01/03/2024", "");
    assert_eq!(income.insights(today()).len(), 0);
}

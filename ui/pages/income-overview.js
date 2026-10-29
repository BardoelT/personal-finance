// Employment income: jobs, yearly totals over all jobs, and the paychecks and annual statements of all jobs (CSD-001).
import { invoke, errorText } from '../shared/ipc.js';
import { h, iconButton, confirmAction, showToast, showError } from '../shared/dom.js';
import { formatAmount, formatDate, currentYear, currentMonth } from '../shared/format.js';

const $ = (id) => document.getElementById(id);

let jobs = [];
let onChanged = async () => {};

function jobDates(job) {
  return `${formatDate(job.start_date)} – ${job.end_date ? formatDate(job.end_date) : 'present'}`;
}

function jobMeta(job) {
  return [job.title, jobDates(job), job.pay_period].filter(Boolean).join(' · ');
}

function amountCell(value) {
  return h('td', { class: 'num' }, formatAmount(value));
}

// ---- Rendering ----

function renderJobs() {
  const hasJobs = jobs.length > 0;
  $('income-empty').hidden = hasJobs;
  $('income-content').hidden = !hasJobs;
  $('job-add').hidden = !hasJobs;
  $('job-list').replaceChildren(
    ...jobs.map((job) =>
      h(
        'li',
        { class: 'job-row' },
        h('span', { class: 'job-name' }, job.employer),
        h('span', { class: 'job-meta' }, jobMeta(job)),
        h(
          'span',
          { class: 'job-actions' },
          iconButton('edit', `Edit ${job.employer}`, () => openJobDialog(job)),
          iconButton('delete', `Delete ${job.employer}`, () => deleteJob(job)),
        ),
      ),
    ),
  );
}

function renderTotals(totals) {
  $('totals-empty').hidden = totals.length > 0;
  $('totals-table').hidden = totals.length === 0;
  $('totals-rows').replaceChildren(
    ...totals.map((t) =>
      h(
        'tr',
        {},
        h('th', { scope: 'row' }, String(t.year)),
        amountCell(t.gross_salary),
        amountCell(t.holiday_pay),
        amountCell(t.bonus),
        amountCell(t.total_gross),
      ),
    ),
  );
}

function renderEntries(entries) {
  $('paycheck-empty').hidden = entries.length > 0;
  $('paycheck-table').hidden = entries.length === 0;
  $('paycheck-rows').replaceChildren(
    ...entries.map((e) =>
      h(
        'tr',
        {},
        h('th', { scope: 'row' }, e.period),
        h('td', { class: 'wrap' }, e.employer),
        amountCell(e.gross_salary),
        amountCell(e.holiday_pay),
        amountCell(e.bonus),
        amountCell(e.total_gross),
        h(
          'td',
          { class: 'row-actions' },
          iconButton('edit', `Edit ${e.employer} ${e.period}`, () => openEntryDialog(e)),
          iconButton('delete', `Delete ${e.employer} ${e.period}`, () => deleteEntry(e)),
        ),
      ),
    ),
  );
}

/** Reloads everything from Rust. */
export async function reloadIncomeOverview() {
  jobs = await invoke('list_jobs');
  renderJobs();
  if (jobs.length === 0) return;
  const [totals, entries] = await Promise.all([invoke('yearly_totals'), invoke('list_entries')]);
  renderTotals(totals);
  renderEntries(entries);
}

async function changed() {
  await reloadIncomeOverview();
  await onChanged();
}

// ---- Job dialog ----

let editingJob = null;

function openJobDialog(job) {
  editingJob = job;
  const form = $('job-form');
  form.reset();
  $('job-error').hidden = true;
  $('job-dialog-title').textContent = job ? 'Edit job' : 'Add job';
  // The pay period is locked once the job has entries (CSD-001 REQ-24).
  const locked = Boolean(job && job.entry_count > 0);
  form.elements.pay_period.disabled = locked;
  $('job-period-hint').hidden = !locked;
  if (job) {
    form.elements.pay_period.value = job.pay_period;
    form.elements.employer.value = job.employer;
    form.elements.title.value = job.title ?? '';
    form.elements.start_date.value = formatDate(job.start_date);
    form.elements.end_date.value = job.end_date ? formatDate(job.end_date) : '';
  }
  $('job-dialog').showModal();
  form.elements.employer.focus();
}

async function submitJob(event) {
  event.preventDefault();
  const form = $('job-form');
  const input = {
    employer: form.elements.employer.value,
    title: form.elements.title.value,
    start_date: form.elements.start_date.value,
    end_date: form.elements.end_date.value,
  };
  if (!form.elements.pay_period.disabled) input.pay_period = form.elements.pay_period.value;
  try {
    if (editingJob) {
      await invoke('update_job', { jobId: editingJob.id, input });
    } else {
      await invoke('create_job', { input });
    }
    $('job-dialog').close();
    await changed();
  } catch (error) {
    showError('job-error', error);
  }
}

async function deleteJob(job) {
  const confirmed = await confirmAction({
    title: `Delete ${job.employer}?`,
    message: 'This job and all its paychecks and statements will be deleted.',
    confirmLabel: 'Delete job',
    danger: true,
  });
  if (!confirmed) return;
  try {
    await invoke('delete_job', { jobId: job.id });
    await changed();
  } catch (error) {
    showToast(errorText(error));
  }
}

// ---- Entry dialog: a paycheck for a monthly job, an annual statement for a yearly job ----

let editingEntry = null;
/** Whether the form currently asks for a year, so the period resets only when it changes. */
let formIsYearly = false;

function fillJobChoices(selectedId) {
  // The list is in job order, so the first option is the newest job (CSD-001 AC-5.6).
  const select = $('paycheck-form').elements.job_id;
  select.replaceChildren(
    ...jobs.map((job) => h('option', { value: job.id }, `${job.employer} · from ${formatDate(job.start_date)}`)),
  );
  select.value = selectedId ?? jobs[0]?.id ?? '';
}

function chosenJob() {
  return jobs.find((job) => job.id === $('paycheck-form').elements.job_id.value);
}

/** Titles and asks for a month or a year, following the chosen job (CSD-001 REQ-28). */
function applyPayPeriod({ resetPeriod }) {
  const form = $('paycheck-form');
  const yearly = chosenJob()?.pay_period === 'Yearly';
  $('paycheck-dialog-title').textContent = yearly ? 'Add annual statement' : 'Add paycheck';
  $('paycheck-period-label').textContent = yearly ? 'Year (YYYY)' : 'Month (YYYY-MM)';
  form.elements.period.placeholder = yearly ? String(new Date().getFullYear() - 1) : '2026-09';
  form.elements.period.maxLength = yearly ? 4 : 7;
  $('paycheck-copy').textContent = yearly ? 'Copy previous year' : 'Copy previous month';
  if (resetPeriod && yearly !== formIsYearly) {
    form.elements.period.value = yearly ? currentYear() : currentMonth();
  }
  formIsYearly = yearly;
}

/** Opens the form to add an entry, or to edit `entry` when given. */
function openEntryDialog(entry = null) {
  editingEntry = entry;
  const form = $('paycheck-form');
  form.reset();
  $('paycheck-error').hidden = true;
  fillJobChoices(entry?.job_id);
  form.elements.job_id.disabled = Boolean(entry);
  $('paycheck-copy').hidden = Boolean(entry);
  if (entry) {
    const statement = entry.kind === 'statement';
    $('paycheck-dialog-title').textContent = statement
      ? `Edit annual statement ${entry.period}`
      : `Edit paycheck ${entry.period}`;
    $('paycheck-period-label').textContent = statement ? 'Year (YYYY)' : 'Month (YYYY-MM)';
    form.elements.period.maxLength = statement ? 4 : 7;
    form.elements.period.value = entry.period;
    form.elements.gross_salary.value = entry.gross_salary;
    form.elements.holiday_pay.value = entry.holiday_pay;
    form.elements.bonus.value = entry.bonus;
  } else {
    formIsYearly = null; // force the period to its default for the first job
    form.elements.period.value = '';
    applyPayPeriod({ resetPeriod: true });
  }
  $('paycheck-dialog').showModal();
  (entry ? form.elements.gross_salary : form.elements.period).focus();
}

async function submitEntry(event) {
  event.preventDefault();
  const form = $('paycheck-form');
  const amounts = {
    gross_salary: form.elements.gross_salary.value,
    holiday_pay: form.elements.holiday_pay.value,
    bonus: form.elements.bonus.value,
  };
  try {
    if (editingEntry) {
      // The entry is found by its current month or year; the field holds the new one (CSD-001 REQ-29).
      const { job_id: jobId, period } = editingEntry;
      const newPeriod = form.elements.period.value;
      if (editingEntry.kind === 'statement') {
        await invoke('update_statement', { jobId, year: period, newYear: newPeriod, amounts });
      } else {
        await invoke('update_paycheck', { jobId, month: period, newMonth: newPeriod, amounts });
      }
    } else {
      const jobId = form.elements.job_id.value;
      const period = form.elements.period.value;
      if (chosenJob()?.pay_period === 'Yearly') {
        await invoke('add_statement', { jobId, year: period, amounts });
      } else {
        await invoke('add_paycheck', { jobId, month: period, amounts });
      }
    }
    $('paycheck-dialog').close();
    await changed();
  } catch (error) {
    showError('paycheck-error', error);
  }
}

/** Fills the form from the latest entry of the job chosen in it (CSD-001 REQ-9). */
async function copyPrevious() {
  const form = $('paycheck-form');
  try {
    const prefill = await invoke('copy_previous_entry', { jobId: form.elements.job_id.value });
    form.elements.gross_salary.value = prefill.gross_salary ?? '';
    form.elements.holiday_pay.value = prefill.holiday_pay;
    form.elements.bonus.value = prefill.bonus;
  } catch (error) {
    showError('paycheck-error', error);
  }
}

async function deleteEntry(entry) {
  try {
    if (entry.kind === 'statement') {
      await invoke('delete_statement', { jobId: entry.job_id, year: entry.period });
    } else {
      await invoke('delete_paycheck', { jobId: entry.job_id, month: entry.period });
    }
    await changed();
  } catch (error) {
    showToast(errorText(error));
  }
}

export function initIncomeOverview({ onChanged: callback }) {
  onChanged = callback;
  $('job-add').addEventListener('click', () => openJobDialog(null));
  $('job-add-first').addEventListener('click', () => openJobDialog(null));
  $('job-form').addEventListener('submit', submitJob);
  $('paycheck-add').addEventListener('click', () => openEntryDialog());
  $('paycheck-copy').addEventListener('click', copyPrevious);
  $('paycheck-form').addEventListener('submit', submitEntry);
  $('paycheck-form').elements.job_id.addEventListener('change', () => applyPayPeriod({ resetPeriod: true }));
}

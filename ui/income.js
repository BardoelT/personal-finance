// Employment income: jobs, yearly totals over all jobs, and the paychecks of all jobs (CSD-001).
import { invoke, errorText, h, formatAmount, formatDate, confirmAction, showToast } from './api.js';

const $ = (id) => document.getElementById(id);

let jobs = [];
let onChanged = async () => {};

function jobDates(job) {
  return `${formatDate(job.start_date)} – ${job.end_date ? formatDate(job.end_date) : 'present'}`;
}

function jobMeta(job) {
  return [job.title, jobDates(job)].filter(Boolean).join(' · ');
}

function currentMonth() {
  const now = new Date();
  return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}`;
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
        h('span', { class: 'job-count' }, `${job.paycheck_count} paycheck${job.paycheck_count === 1 ? '' : 's'}`),
        h(
          'span',
          { class: 'job-actions' },
          h('button', { type: 'button', class: 'btn', onClick: () => openJobDialog(job) }, 'Edit job'),
          h('button', { type: 'button', class: 'btn danger', onClick: () => deleteJob(job) }, 'Delete job'),
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
        h('td', { class: 'num' }, String(t.paycheck_count)),
      ),
    ),
  );
}

function renderPaychecks(paychecks) {
  $('paycheck-empty').hidden = paychecks.length > 0;
  $('paycheck-table').hidden = paychecks.length === 0;
  $('paycheck-rows').replaceChildren(
    ...paychecks.map((p) =>
      h(
        'tr',
        {},
        h('th', { scope: 'row' }, p.month),
        h('td', { class: 'wrap' }, p.employer),
        amountCell(p.gross_salary),
        amountCell(p.holiday_pay),
        amountCell(p.bonus),
        amountCell(p.total_gross),
        h(
          'td',
          { class: 'row-actions' },
          h('button', { type: 'button', class: 'link-btn', onClick: () => openPaycheckDialog(p) }, 'Edit'),
          h('button', { type: 'button', class: 'link-btn danger', onClick: () => deletePaycheck(p) }, 'Delete'),
        ),
      ),
    ),
  );
}

/** Reloads everything from Rust. */
export async function reloadIncome() {
  jobs = await invoke('list_jobs');
  const { currency } = await invoke('get_settings');
  $('totals-title').textContent = `Yearly totals · ${currency}`;
  $('paycheck-title').textContent = `Paychecks · ${currency}`;
  renderJobs();
  if (jobs.length === 0) return;
  const [totals, paychecks] = await Promise.all([invoke('yearly_totals'), invoke('list_paychecks')]);
  renderTotals(totals);
  renderPaychecks(paychecks);
}

async function changed() {
  await reloadIncome();
  await onChanged();
}

function showError(id, error) {
  const box = $(id);
  box.textContent = errorText(error);
  box.hidden = false;
}

// ---- Job dialog ----

let editingJob = null;

function openJobDialog(job) {
  editingJob = job;
  const form = $('job-form');
  form.reset();
  $('job-error').hidden = true;
  $('job-dialog-title').textContent = job ? 'Edit job' : 'Add job';
  if (job) {
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
  const n = job.paycheck_count;
  const confirmed = await confirmAction({
    title: `Delete ${job.employer}?`,
    message:
      n === 0
        ? 'This job will be deleted.'
        : `This job and its ${n} paycheck${n === 1 ? '' : 's'} will be deleted.`,
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

// ---- Paycheck dialog ----

let editingPaycheck = null;

function fillJobChoices(selectedId) {
  // The list is in job order, so the first option is the newest job (CSD-001 AC-5.6).
  const select = $('paycheck-form').elements.job_id;
  select.replaceChildren(
    ...jobs.map((job) => h('option', { value: job.id }, `${job.employer} · from ${formatDate(job.start_date)}`)),
  );
  select.value = selectedId ?? jobs[0]?.id ?? '';
}

/** Opens the form to add a paycheck, or to edit `paycheck` when given. */
function openPaycheckDialog(paycheck = null) {
  editingPaycheck = paycheck;
  const form = $('paycheck-form');
  form.reset();
  $('paycheck-error').hidden = true;
  $('paycheck-dialog-title').textContent = paycheck ? `Edit paycheck ${paycheck.month}` : 'Add paycheck';
  fillJobChoices(paycheck?.job_id);
  form.elements.job_id.disabled = Boolean(paycheck);
  $('paycheck-copy').hidden = Boolean(paycheck);
  form.elements.month.readOnly = Boolean(paycheck);
  if (paycheck) {
    form.elements.month.value = paycheck.month;
    form.elements.gross_salary.value = paycheck.gross_salary;
    form.elements.holiday_pay.value = paycheck.holiday_pay;
    form.elements.bonus.value = paycheck.bonus;
  } else {
    form.elements.month.value = currentMonth();
  }
  $('paycheck-dialog').showModal();
  (paycheck ? form.elements.gross_salary : form.elements.month).focus();
}

async function submitPaycheck(event) {
  event.preventDefault();
  const form = $('paycheck-form');
  const amounts = {
    gross_salary: form.elements.gross_salary.value,
    holiday_pay: form.elements.holiday_pay.value,
    bonus: form.elements.bonus.value,
  };
  try {
    if (editingPaycheck) {
      await invoke('update_paycheck', {
        jobId: editingPaycheck.job_id,
        month: editingPaycheck.month,
        amounts,
      });
    } else {
      await invoke('add_paycheck', {
        jobId: form.elements.job_id.value,
        month: form.elements.month.value,
        amounts,
      });
    }
    $('paycheck-dialog').close();
    await changed();
  } catch (error) {
    showError('paycheck-error', error);
  }
}

/** Fills the form from the latest paycheck of the job chosen in it (CSD-001 REQ-9). */
async function copyPrevious() {
  const form = $('paycheck-form');
  try {
    const prefill = await invoke('copy_previous_paycheck', { jobId: form.elements.job_id.value });
    form.elements.gross_salary.value = prefill.gross_salary ?? '';
    form.elements.holiday_pay.value = prefill.holiday_pay;
    form.elements.bonus.value = prefill.bonus;
  } catch (error) {
    showError('paycheck-error', error);
  }
}

async function deletePaycheck(paycheck) {
  try {
    await invoke('delete_paycheck', { jobId: paycheck.job_id, month: paycheck.month });
    await changed();
  } catch (error) {
    showToast(errorText(error));
  }
}

export function initIncome({ onChanged: callback }) {
  onChanged = callback;
  $('job-add').addEventListener('click', () => openJobDialog(null));
  $('job-add-first').addEventListener('click', () => openJobDialog(null));
  $('job-form').addEventListener('submit', submitJob);
  $('paycheck-add').addEventListener('click', () => openPaycheckDialog());
  $('paycheck-copy').addEventListener('click', copyPrevious);
  $('paycheck-form').addEventListener('submit', submitPaycheck);
}

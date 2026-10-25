// Employment income: jobs, monthly paychecks and yearly totals (CSD-001).
import { invoke, errorText, h, formatAmount, confirmAction, showToast } from './api.js';

const $ = (id) => document.getElementById(id);

let jobs = [];
let selectedId = null;
let onChanged = async () => {};

function selectedJob() {
  return jobs.find((job) => job.id === selectedId) ?? null;
}

function jobDates(job) {
  return `${job.start_date} – ${job.end_date ?? 'present'}`;
}

function jobMeta(job) {
  return [job.title, jobDates(job), job.currency].filter(Boolean).join(' · ');
}

function currentMonth() {
  const now = new Date();
  return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}`;
}

function renderJobs() {
  const hasJobs = jobs.length > 0;
  $('income-empty').hidden = hasJobs;
  $('jobs-panel').hidden = !hasJobs;
  $('job-detail').hidden = !hasJobs;
  $('job-add').hidden = !hasJobs;
  const list = $('job-list');
  list.replaceChildren(
    ...jobs.map((job) =>
      h(
        'li',
        {},
        h(
          'button',
          {
            type: 'button',
            class: 'job-row',
            'aria-pressed': String(job.id === selectedId),
            onClick: () => {
              selectedId = job.id;
              renderJobs();
              loadDetail();
            },
          },
          h('span', { class: 'job-name' }, job.employer),
          h('span', { class: 'job-meta' }, jobMeta(job)),
          h('span', { class: 'job-count' }, `${job.paycheck_count} paycheck${job.paycheck_count === 1 ? '' : 's'}`),
        ),
      ),
    ),
  );
  const job = selectedJob();
  if (job) {
    $('job-detail-title').textContent = job.employer;
    $('job-detail-meta').textContent = jobMeta(job);
    $('paycheck-title').textContent = `Paychecks · amounts in ${job.currency}`;
    $('totals-title').textContent = `Yearly totals · amounts in ${job.currency}`;
  }
}

function amountCell(value) {
  return h('td', { class: 'num' }, formatAmount(value));
}

async function loadDetail() {
  const job = selectedJob();
  if (!job) return;
  const [paychecks, totals] = await Promise.all([
    invoke('list_paychecks', { jobId: job.id }),
    invoke('yearly_totals', { jobId: job.id }),
  ]);
  if (selectedId !== job.id) return; // another job was selected meanwhile

  $('paycheck-empty').hidden = paychecks.length > 0;
  $('paycheck-table').hidden = paychecks.length === 0;
  $('paycheck-rows').replaceChildren(
    ...paychecks.map((p) =>
      h(
        'tr',
        {},
        h('th', { scope: 'row' }, p.month),
        amountCell(p.gross_salary),
        amountCell(p.holiday_pay),
        amountCell(p.bonus),
        amountCell(p.total_gross),
        h(
          'td',
          { class: 'row-actions' },
          h('button', { type: 'button', class: 'link-btn', onClick: () => openPaycheckDialog({ paycheck: p }) }, 'Edit'),
          h('button', { type: 'button', class: 'link-btn danger', onClick: () => deletePaycheck(p) }, 'Delete'),
        ),
      ),
    ),
  );

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

/** Reloads everything from Rust, keeping the selected job if it still exists. */
export async function reloadIncome() {
  jobs = await invoke('list_jobs');
  if (!selectedJob()) selectedId = jobs[0]?.id ?? null;
  renderJobs();
  await loadDetail();
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
  const locked = Boolean(job && job.paycheck_count > 0);
  form.elements.currency.disabled = locked;
  $('job-currency-hint').hidden = !locked;
  if (job) {
    form.elements.employer.value = job.employer;
    form.elements.title.value = job.title ?? '';
    form.elements.start_date.value = job.start_date;
    form.elements.end_date.value = job.end_date ?? '';
    form.elements.currency.value = job.currency;
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
  if (!form.elements.currency.disabled) input.currency = form.elements.currency.value;
  try {
    const saved = editingJob
      ? await invoke('update_job', { jobId: editingJob.id, input })
      : await invoke('create_job', { input });
    selectedId = saved.id;
    $('job-dialog').close();
    await changed();
  } catch (error) {
    showError('job-error', error);
  }
}

async function deleteJob() {
  const job = selectedJob();
  if (!job) return;
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
    selectedId = null;
    await changed();
  } catch (error) {
    showToast(errorText(error));
  }
}

// ---- Paycheck dialog ----

let editingPaycheck = null;

function openPaycheckDialog({ paycheck = null, prefill = null } = {}) {
  editingPaycheck = paycheck;
  const form = $('paycheck-form');
  form.reset();
  $('paycheck-error').hidden = true;
  $('paycheck-dialog-title').textContent = paycheck ? `Edit paycheck ${paycheck.month}` : 'Add paycheck';
  form.elements.month.readOnly = Boolean(paycheck);
  if (paycheck) {
    form.elements.month.value = paycheck.month;
    form.elements.gross_salary.value = paycheck.gross_salary;
    form.elements.holiday_pay.value = paycheck.holiday_pay;
    form.elements.bonus.value = paycheck.bonus;
  } else {
    form.elements.month.value = currentMonth();
    if (prefill) {
      form.elements.gross_salary.value = prefill.gross_salary ?? '';
      form.elements.holiday_pay.value = prefill.holiday_pay;
      form.elements.bonus.value = prefill.bonus;
    }
  }
  $('paycheck-dialog').showModal();
  (paycheck ? form.elements.gross_salary : form.elements.month).focus();
}

async function submitPaycheck(event) {
  event.preventDefault();
  const job = selectedJob();
  if (!job) return;
  const form = $('paycheck-form');
  const amounts = {
    gross_salary: form.elements.gross_salary.value,
    holiday_pay: form.elements.holiday_pay.value,
    bonus: form.elements.bonus.value,
  };
  const month = form.elements.month.value;
  try {
    if (editingPaycheck) {
      await invoke('update_paycheck', { jobId: job.id, month: editingPaycheck.month, amounts });
    } else {
      await invoke('add_paycheck', { jobId: job.id, month, amounts });
    }
    $('paycheck-dialog').close();
    await changed();
  } catch (error) {
    showError('paycheck-error', error);
  }
}

async function copyPrevious() {
  const job = selectedJob();
  if (!job) return;
  try {
    const prefill = await invoke('copy_previous_paycheck', { jobId: job.id });
    openPaycheckDialog({ prefill });
  } catch (error) {
    showToast(errorText(error));
  }
}

async function deletePaycheck(paycheck) {
  const job = selectedJob();
  if (!job) return;
  try {
    await invoke('delete_paycheck', { jobId: job.id, month: paycheck.month });
    await changed();
  } catch (error) {
    showToast(errorText(error));
  }
}

export function initIncome({ onChanged: callback }) {
  onChanged = callback;
  $('job-add').addEventListener('click', () => openJobDialog(null));
  $('job-add-first').addEventListener('click', () => openJobDialog(null));
  $('job-edit').addEventListener('click', () => openJobDialog(selectedJob()));
  $('job-delete').addEventListener('click', deleteJob);
  $('job-form').addEventListener('submit', submitJob);
  $('paycheck-add').addEventListener('click', () => openPaycheckDialog());
  $('paycheck-copy').addEventListener('click', copyPrevious);
  $('paycheck-form').addEventListener('submit', submitPaycheck);
}

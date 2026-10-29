// Asset insights: the yearly stacked chart on the Assets page (CSD-016), one
// segment per asset. Its frame (axis, bars, tooltip, loading) is shared with the
// other charts in shared/yearly-chart.js; this file supplies what is specific
// to assets: the segments, the legend of assets and the total value marker.
import { h } from '../shared/dom.js';
import { formatAmount } from '../shared/format.js';
import { swatch, noSwatch, legendLabel, nameLabel, legendItem, tooltipRow, topToBottom } from '../shared/chart.js';
import { createYearlyChart, levelMarker, isNegative, yearLabel } from '../shared/yearly-chart.js';
import { initTabs } from '../shared/tabs.js';

/** The assets of the series by id, as Rust returned them (creation order). */
let items = new Map();

const keyOf = (id) => `asset-${id}`;
const nameOf = (segment) => items.get(segment.asset_id)?.name ?? '';
const slotOf = (id) => items.get(id)?.colour_slot ?? 0;
/** The change amount as shown: a dash when Rust has none. */
const deltaText = (row) => (row.change === null ? '–' : formatAmount(row.change));

function describe(row) {
  if (row.valuation_count === 0) return `${yearLabel(row)}: no valuations`;
  const parts = row.segments.map((segment) => `${nameOf(segment)} ${formatAmount(segment.value)}`);
  const delta = row.change === null ? 'change none' : `change ${formatAmount(row.change)}${isNegative(row.change) ? ' (negative)' : ''}`;
  return `${yearLabel(row)}: ${parts.join(', ')}, total value ${formatAmount(row.closing_value)}, ${delta}`;
}

/** The rows read like the bar: the highest segment first. */
function tooltipContent(row) {
  const rows =
    row.valuation_count === 0
      ? []
      : [
          ...topToBottom(row.segments, (segment) => segment.value).flatMap((segment) =>
            tooltipRow(keyOf(segment.asset_id), swatch(`slot-${slotOf(segment.asset_id)}`), nameOf(segment), formatAmount(segment.value)),
          ),
          ...tooltipRow('marker', swatch('marker'), 'Total value', formatAmount(row.closing_value)),
          ...tooltipRow(null, noSwatch(), 'Change', deltaText(row), row.change !== null && isNegative(row.change) ? 'negative' : ''),
        ];
  return [
    h('div', { class: 'tip-title' }, yearLabel(row)),
    row.is_partial ? h('div', { class: 'tip-note' }, 'Partial year: valuations so far') : null,
    row.valuation_count === 0 ? h('div', { class: 'tip-note' }, 'No valuations') : null,
    h('dl', {}, ...rows),
  ].filter(Boolean);
}

/**
 * The segments of a year, stacked upwards from the zero line in the order
 * received (CSD-016 REQ-6). A value is never negative, so nothing hangs down;
 * a asset at 0.00 adds no height. The top corners of the highest segment are rounded.
 */
function piecesOf(row) {
  let level = 0;
  const pieces = [];
  for (const segment of row.segments) {
    const value = Number(segment.value);
    if (value === 0) continue;
    pieces.push({
      key: keyOf(segment.asset_id),
      cssClass: `bar-slot-${slotOf(segment.asset_id)}`,
      label: `${nameOf(segment)}, ${row.year}, ${formatAmount(segment.value)}`,
      low: level,
      high: level + value,
      hangs: false,
      rounding: false,
    });
    level += value;
  }
  const highest = pieces.at(-1);
  if (highest) highest.rounding = 'top';
  return pieces;
}

/** The legend: a swatch and the name of every asset in the series, and the total value marker (CSD-016 REQ-9). */
function buildLegend({ assets: list }) {
  items = new Map(list.map((item) => [item.id, item]));
  const legendList = document.getElementById('assets-insights-legend');
  const scopes = () => [legendList, document.getElementById('assets-insights-chart')];
  legendList.replaceChildren(
    ...list.map((item) => legendItem(keyOf(item.id), swatch(`slot-${item.colour_slot}`), nameLabel(item.name), scopes)),
    legendItem('marker', swatch('marker'), legendLabel('Total value'), scopes),
  );
}

const chart = createYearlyChart({
  id: 'assets-insights',
  command: 'get_asset_insights',
  ariaLabel: 'Assets per year',
  onLoaded: buildLegend,
  piecesOf,
  extraLevels: (row) => [Number(row.closing_value)],
  describe,
  tooltipContent,
  /** The total value marker, drawn for each year that has valuations. */
  decorate: (row, geometry) => (row.valuation_count === 0 ? [] : levelMarker(Number(row.closing_value), geometry)),
  labelLines: (row) => [{ text: yearLabel(row), className: 'chart-year' }],
});

export const reloadAssetsInsights = chart.reload;
/** Reloads the chart only while the Insights tab is showing (after a project is opened). */
export const reloadAssetsInsightsIfOpen = chart.reloadIfOpen;

export function initAssetsInsights() {
  chart.init();
  initTabs([
    { tab: 'assets-tab-overview', panel: 'assets-overview' },
    // Opening Insights recomputes it, so it never shows stale figures (CSD-016 REQ-15).
    { tab: 'assets-tab-insights', panel: 'assets-insights', onOpen: chart.reload },
  ]);
}

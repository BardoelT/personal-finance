// House insights: the yearly stacked chart on the Houses page (CSD-016), one
// segment per house. Its frame (axis, bars, tooltip, loading) is shared with the
// other charts in shared/yearly-chart.js; this file supplies what is specific
// to houses: the segments, the legend of houses and the total value marker.
import { h } from '../shared/dom.js';
import { formatAmount } from '../shared/format.js';
import { swatch, noSwatch, legendLabel, nameLabel, legendItem, tooltipRow, topToBottom } from '../shared/chart.js';
import { createYearlyChart, levelMarker, isNegative, yearLabel } from '../shared/yearly-chart.js';
import { initTabs } from '../shared/tabs.js';

/** The houses of the series by id, as Rust returned them (creation order). */
let items = new Map();

const keyOf = (id) => `house-${id}`;
const nameOf = (segment) => items.get(segment.house_id)?.name ?? '';
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
            tooltipRow(keyOf(segment.house_id), swatch(`slot-${slotOf(segment.house_id)}`), nameOf(segment), formatAmount(segment.value)),
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
 * a house at 0.00 adds no height. The top corners of the highest segment are rounded.
 */
function piecesOf(row) {
  let level = 0;
  const pieces = [];
  for (const segment of row.segments) {
    const value = Number(segment.value);
    if (value === 0) continue;
    pieces.push({
      key: keyOf(segment.house_id),
      cssClass: `bar-slot-${slotOf(segment.house_id)}`,
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

/** The legend: a swatch and the name of every house in the series, and the total value marker (CSD-016 REQ-9). */
function buildLegend({ houses: list }) {
  items = new Map(list.map((item) => [item.id, item]));
  const legendList = document.getElementById('houses-insights-legend');
  const scopes = () => [legendList, document.getElementById('houses-insights-chart')];
  legendList.replaceChildren(
    ...list.map((item) => legendItem(keyOf(item.id), swatch(`slot-${item.colour_slot}`), nameLabel(item.name), scopes)),
    legendItem('marker', swatch('marker'), legendLabel('Total value'), scopes),
  );
}

const chart = createYearlyChart({
  id: 'houses-insights',
  command: 'get_house_insights',
  ariaLabel: 'Houses per year',
  onLoaded: buildLegend,
  piecesOf,
  extraLevels: (row) => [Number(row.closing_value)],
  describe,
  tooltipContent,
  /** The total value marker, drawn for each year that has valuations. */
  decorate: (row, geometry) => (row.valuation_count === 0 ? [] : levelMarker(Number(row.closing_value), geometry)),
  labelLines: (row) => [{ text: yearLabel(row), className: 'chart-year' }],
});

/** Reloads the chart only while the Insights tab is showing (after a project is opened). */
export const reloadHousesInsightsIfOpen = chart.reloadIfOpen;

export function initHousesInsights() {
  chart.init();
  initTabs([
    { tab: 'houses-tab-overview', panel: 'houses-overview' },
    // Opening Insights recomputes it, so it never shows stale figures (CSD-016 REQ-15).
    { tab: 'houses-tab-insights', panel: 'houses-insights', onOpen: chart.reload },
  ]);
}

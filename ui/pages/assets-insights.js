// Asset insights: the yearly stacked chart on the Assets page (CSD-016), one
// segment per asset. Its frame (axis, bars, tooltip, loading) is shared with the
// other charts in shared/yearly-chart.js; this file supplies what is specific
// to assets: the segments, the legend of assets and the total-value marker.
import { h } from '../shared/dom.js';
import { formatAmount } from '../shared/format.js';
import { svg, swatch, noSwatch, legendLabel, setHighlight } from '../shared/chart.js';
import { createYearlyChart, isNegative, yearLabel, axisLabel } from '../shared/yearly-chart.js';
import { initTabs } from '../shared/tabs.js';

/** The assets of the series by id, as Rust returned them (creation order). */
let assets = new Map();

const keyOf = (assetId) => `asset-${assetId}`;
const nameOf = (segment) => assets.get(segment.asset_id)?.name ?? '';
const slotOf = (assetId) => assets.get(assetId)?.colour_slot ?? 0;
const valuationsText = (count) => `${count} valuation${count === 1 ? '' : 's'}`;
/** The change amount as shown: a dash when Rust has none. */
const changeText = (row) => (row.change === null ? '–' : formatAmount(row.change));

function describe(row) {
  if (row.valuation_count === 0) return `${yearLabel(row)}: no valuations`;
  const parts = row.segments.map((segment) => `${nameOf(segment)} ${formatAmount(segment.value)}`);
  const change = row.change === null ? 'change none' : `change ${formatAmount(row.change)}${isNegative(row.change) ? ' (negative)' : ''}`;
  return `${yearLabel(row)}: ${parts.join(', ')}, total value ${formatAmount(row.closing_value)}, ${change}, ${valuationsText(row.valuation_count)}`;
}

function tooltipContent(row) {
  const line = (legend, swatchNode, label, value, cssClass = '') => [
    h('dt', { 'data-legend': legend }, swatchNode, legendLabel(label)),
    h('dd', { 'data-legend': legend, class: cssClass }, value),
  ];
  const rows =
    row.valuation_count === 0
      ? line(null, noSwatch(), 'Valuations', '0')
      : [
          ...row.segments.flatMap((segment) =>
            line(keyOf(segment.asset_id), swatch(`slot-${slotOf(segment.asset_id)}`), nameOf(segment), formatAmount(segment.value)),
          ),
          ...line('marker', swatch('marker'), 'Total value', formatAmount(row.closing_value)),
          ...line(null, noSwatch(), 'Change', changeText(row), row.change !== null && isNegative(row.change) ? 'negative' : ''),
          ...line(null, noSwatch(), 'Valuations', String(row.valuation_count)),
        ];
  return [
    h('div', { class: 'tip-title' }, yearLabel(row)),
    row.is_partial ? h('div', { class: 'tip-note' }, 'Partial year: valuations so far') : null,
    h('dl', {}, ...rows),
  ].filter(Boolean);
}

/**
 * The segments of a year, stacked upwards from the zero line in the order
 * received (CSD-016 REQ-6). A value is never negative, so nothing hangs down;
 * an asset at 0.00 adds no height. The top corners of the highest segment are rounded.
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

/** The legend: a swatch and the name of every asset in the series, and the total-value marker (CSD-016 REQ-9). */
function buildLegend({ assets: list }) {
  assets = new Map(list.map((asset) => [asset.id, asset]));
  const legendList = document.getElementById('assets-insights-legend');
  const scopes = () => [legendList, document.getElementById('assets-insights-chart')];
  const items = list.map((asset) => {
    const item = h(
      'li',
      { 'data-legend': keyOf(asset.id), tabindex: '0' },
      swatch(`slot-${asset.colour_slot}`),
      h('span', { class: 'legend-label legend-name', 'data-text': asset.name }, h('span', { class: 'legend-name-text' }, asset.name)),
    );
    // Hovering or focusing an asset highlights it in every year (CSD-016 REQ-11).
    for (const on of ['mouseenter', 'focus']) item.addEventListener(on, () => setHighlight(keyOf(asset.id), scopes()));
    for (const off of ['mouseleave', 'blur']) item.addEventListener(off, () => setHighlight(null, scopes()));
    return item;
  });
  items.push(h('li', { 'data-legend': 'marker' }, swatch('marker'), legendLabel('Total value')));
  legendList.replaceChildren(...items);
}

const chart = createYearlyChart({
  id: 'assets-insights',
  command: 'get_asset_insights',
  ariaLabel: 'Assets per year',
  plotHeight: 260,
  minSlotWidth: 64,
  zeroLine: true,
  highlightAcrossYears: true,
  onLoaded: buildLegend,
  piecesOf,
  extraLevels: (row) => [Number(row.closing_value)],
  describe,
  tooltipContent,
  /** The total-value marker, drawn for each year that has valuations. */
  decorate(row, { centre, barWidth, yOf }) {
    if (row.valuation_count === 0) return [];
    const y = yOf(Number(row.closing_value));
    const half = barWidth * 0.7;
    return [
      svg('line', { class: 'chart-marker-hit', 'data-legend': 'marker', x1: centre - half, x2: centre + half, y1: y, y2: y }),
      svg('line', { class: 'chart-marker', 'data-legend': 'marker', x1: centre - half, x2: centre + half, y1: y, y2: y }),
      svg('text', { class: 'chart-marker-label', x: centre, y: y - 7 }, axisLabel(Number(row.closing_value))),
    ];
  },
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

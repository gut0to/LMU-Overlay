import type { WidgetLayout } from "./types";
import { scaledDimension } from "./geometry.ts";

/** Choose the globally closest alignment from the original position. */
export function snapToWidgetLayouts(layout: WidgetLayout, others: WidgetLayout[], distance: number): WidgetLayout {
  const width = scaledDimension(layout.width, layout.scale);
  const height = scaledDimension(layout.height, layout.scale);
  const xTargets: number[] = [];
  const yTargets: number[] = [];
  for (const other of others) {
    const right = other.x + scaledDimension(other.width, other.scale);
    const bottom = other.y + scaledDimension(other.height, other.scale);
    xTargets.push(other.x, right, other.x - width, right - width);
    yTargets.push(other.y, bottom, other.y - height, bottom - height);
  }
  const nearest = (value: number, targets: number[]) => targets.reduce((best, target) => {
    const gap = Math.abs(target - value);
    const bestGap = Math.abs(best - value);
    return gap <= distance && (gap < bestGap || (gap === bestGap && target < best)) ? target : best;
  }, targets[0]);
  // Empty candidate sets preserve the original coordinate.
  const align = (value: number, targets: number[]) => {
    const valid = targets.filter((target) => Number.isFinite(target) && Math.abs(target - value) <= distance);
    if (valid.length === 0) return value;
    return nearest(value, valid);
  };
  return { ...layout, x: align(layout.x, xTargets), y: align(layout.y, yTargets) };
}

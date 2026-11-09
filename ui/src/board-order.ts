import type { Card, Column, Space } from "./workspace-api";

export type MoveTarget = { kind: "card.move"; target: Card } | { kind: "column.move"; target: Column };
export type Placement = { column: string; before: string | null };
export const objectTitle = (item: Column | Space) => item.names.length === 1 ? item.names[0] || "Без названия" : "Конфликт названий";
export const cardTitle = (item: Card) => item.titles.length === 1 ? item.titles[0]! : "Конфликт заголовков";
export const columnConflict = (item: Column | Space) => item.state === "conflicted" || item.names.length !== 1 || "positions" in item && item.positions.length !== 1;
export const movableCard = (item: Card) => item.state === "active" && item.titles.length === 1 && item.descriptions.length === 1 && item.columns.length === 1 && item.positions.length === 1 && item.completed.length === 1;
// Sortable owns only this disposable presentation; ranks are exclusively Rust data.
export const dragOptions = { animation: 0, delay: 250, delayOnTouchOnly: true, touchStartThreshold: 5,
  forceFallback: true, fallbackOnBody: true, scroll: true, bubbleScroll: true, scrollSensitivity: 50, scrollSpeed: 12 };

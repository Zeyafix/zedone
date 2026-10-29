import type { StorageRequest, StorageResponse } from "./storage";

export type Snapshot = NonNullable<StorageResponse["snapshot"]>;
export type Move = Extract<StorageRequest, { action: "move_card" | "move_column" }>;
export type Target = { kind: "card" | "column"; id: number };

export function inverse(move: Move, after: Snapshot): Move {
  if (move.action === "move_column") {
    return { action: move.action, expected: after, column: move.position, position: move.column };
  }
  const card = move.expected.cards.find(card => card.id === move.card)!;
  return { action: move.action, expected: after, card: card.id, column: card.column,
    position: move.expected.cards.filter(item => item.column === card.column).findIndex(item => item.id === card.id) };
}

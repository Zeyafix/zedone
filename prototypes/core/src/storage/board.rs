//! P01.06 only: compare the complete synthetic snapshot inside the write transaction.
use super::{Snapshot, load};
use rusqlite::{Connection, Error, Result};

fn invalid(message: &str) -> Error {
    Error::InvalidParameterName(message.into())
}

pub(super) fn check(db: &Connection, expected: &Snapshot) -> Result<()> {
    if load(db, expected.space)?.as_ref() != Some(expected) {
        return Err(invalid(
            "Доска изменилась. Прочитайте данные и подготовьте перемещение заново.",
        ));
    }
    Ok(())
}

pub(super) fn move_card(
    snapshot: &mut Snapshot,
    id: u32,
    column: u32,
    position: u32,
) -> Result<()> {
    let source = snapshot
        .cards
        .iter()
        .position(|card| card.id == id)
        .ok_or_else(|| invalid("Карточка отсутствует."))?;
    let count = snapshot
        .cards
        .iter()
        .filter(|card| card.column == column && card.id != id)
        .count();
    if column as usize >= snapshot.columns.len() || position as usize > count {
        return Err(invalid("Недопустимая позиция карточки."));
    }
    let mut card = snapshot.cards.remove(source);
    card.column = column;
    // Insert relative to the destination AFTER removing the source, including empty lists.
    let index = snapshot
        .cards
        .iter()
        .enumerate()
        .filter(|(_, card)| card.column == column)
        .nth(position as usize)
        .map(|(index, _)| index)
        .unwrap_or_else(|| {
            snapshot
                .cards
                .iter()
                .position(|card| card.column > column)
                .unwrap_or(snapshot.cards.len())
        });
    snapshot.cards.insert(index, card);
    Ok(())
}

pub(super) fn move_column(snapshot: &mut Snapshot, column: u32, position: u32) -> Result<()> {
    let mut order: Vec<usize> = (0..snapshot.columns.len()).collect();
    if column as usize >= order.len() || position as usize >= order.len() {
        return Err(invalid("Недопустимая позиция колонки."));
    }
    let source = order.remove(column as usize);
    order.insert(position as usize, source);
    snapshot.columns = order
        .iter()
        .map(|&index| snapshot.columns[index].clone())
        .collect();
    for card in &mut snapshot.cards {
        card.column = order
            .iter()
            .position(|&index| index == card.column as usize)
            .ok_or_else(|| invalid("Колонка карточки отсутствует."))? as u32;
    }
    snapshot.cards.sort_by_key(|card| card.column);
    Ok(())
}

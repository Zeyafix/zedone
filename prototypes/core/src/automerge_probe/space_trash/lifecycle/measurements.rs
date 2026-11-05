//! Release workload measurements. No application memory/latency acceptance claim.
use super::*;
use std::time::Instant;

pub(in super::super::super) fn stats(samples: &[f64]) -> Value {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    json!({"samples_ms": samples, "min_ms": sorted[0],
        "median_ms": sorted[sorted.len()/2], "max_ms": sorted[sorted.len()-1]})
}

pub(in super::super::super) fn timed<T>(f: impl FnOnce() -> Result<T>) -> Result<(T, f64)> {
    let start = Instant::now();
    let result = f()?;
    Ok((result, start.elapsed().as_secs_f64() * 1000.0))
}

fn memory() -> Result<Value> {
    #[cfg(windows)]
    {
        let output = std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", &format!(
                "Get-Process -Id {} | ForEach-Object {{ [ordered]@{{ peak_working_set_bytes=$_.PeakWorkingSet64; working_set_bytes=$_.WorkingSet64; private_bytes=$_.PrivateMemorySize64 }} | ConvertTo-Json -Compress }}",
                std::process::id())]).output()?;
        if !output.status.success() {
            return Err("Could not read Windows process memory counters".into());
        }
        Ok(serde_json::from_slice(&output.stdout)?)
    }
    #[cfg(not(windows))]
    {
        let status = std::fs::read_to_string("/proc/self/status")?;
        let value = |name: &str| -> Result<u64> {
            let line = status
                .lines()
                .find(|l| l.starts_with(name))
                .ok_or("Missing process memory counter")?;
            Ok(line
                .split_whitespace()
                .nth(1)
                .ok_or("Missing memory value")?
                .parse::<u64>()?
                * 1024)
        };
        Ok(
            json!({"peak_working_set_bytes": value("VmHWM:")?, "working_set_bytes": value("VmRSS:")?,
            "virtual_peak_bytes": value("VmPeak:")?}),
        )
    }
}

fn text(version: usize, lines: usize) -> String {
    (0..lines)
        .map(|line| {
            format!(
                "{line:03}: Сохранение {version:06} — ёж, cafe\u{301}, 😀, Markdown **заметка**\n"
            )
        })
        .collect()
}

fn exchange_size(mut a: Replica, mut b: Replica) -> Result<Value> {
    a.peer = State::new();
    b.peer = State::new();
    let mut bytes = 0;
    let mut messages = 0;
    for round in 0..16 {
        let mut sent = false;
        if let Some(message) = a.doc.generate_sync_message(&mut a.peer) {
            let encoded = message.encode();
            bytes += encoded.len();
            messages += 1;
            b.doc
                .receive_sync_message(&mut b.peer, automerge::sync::Message::decode(&encoded)?)?;
            sent = true;
        }
        if let Some(message) = b.doc.generate_sync_message(&mut b.peer) {
            let encoded = message.encode();
            bytes += encoded.len();
            messages += 1;
            a.doc
                .receive_sync_message(&mut a.peer, automerge::sync::Message::decode(&encoded)?)?;
            sent = true;
        }
        if !sent {
            if a.doc.get_heads() != b.doc.get_heads() {
                return Err("Measurement sync diverged".into());
            }
            return Ok(json!({"wire_bytes": bytes, "messages": messages, "rounds": round}));
        }
    }
    Err("Measurement sync failed to settle".into())
}

fn hierarchy(directory: &Path, cards: usize, edits: usize, lines: usize) -> Result<Value> {
    let path = directory.join("measurement.sqlite3");
    let mut store = Store::open(&path)?;
    seed(&mut store)?;
    let mut a = store.read("A")?;
    let root = a.doc.get(ROOT, "nodes")?.ok_or("Missing nodes")?.1;
    let start = Instant::now();
    let mut tx = a.doc.transaction();
    for i in 0..cards {
        add(
            &mut tx,
            &root,
            &format!("bench-{i}"),
            "card",
            "c0",
            "active",
        )?;
    }
    tx.commit();
    let create_ms = start.elapsed().as_secs_f64() * 1000.0;
    let checkpoint = a.doc.get_heads();
    let mut incremental_bytes = a.doc.save().len();
    let mut edits_ms = vec![];
    let mut save_ms = vec![];
    for i in 0..edits {
        let heads = a.doc.get_heads();
        let body = text(i, lines);
        let (_, ms) =
            timed(|| history::save_pair(&mut a.doc, "bench-0", &format!("Версия {i}"), &body))?;
        edits_ms.push(ms);
        incremental_bytes += a.doc.save_after(&heads).len();
        if (i + 1) % (edits / 5).max(1) == 0 {
            save_ms.push(timed(|| store.write("A", &a))?.1);
        }
    }
    store.write("A", &a)?;
    let packed = a.doc.save();
    let mut load_ms = vec![];
    let mut projection_ms = vec![];
    for _ in 0..7 {
        let (loaded, ms) = timed(|| Ok(Automerge::load(&packed)?))?;
        load_ms.push(ms);
        projection_ms.push(
            timed(|| {
                view(&loaded)?;
                history::versions(&loaded)?;
                Ok(())
            })?
            .1,
        );
        save_ms.push(timed(|| store.write("A", &a))?.1);
    }
    let snapshot_ms = timed(|| save_snapshot(&mut store, "workload", &a.doc))?.1;
    let wire = exchange_size(store.read("A")?, store.read("B")?)?;
    let sync_ms = timed(|| sync(&mut store))?.1;
    a = store.read("A")?;
    let before_clear_changes = a.doc.get_changes(&[]).len();
    let history_count = history::versions(&a.doc)?.len();
    let plan = history::preview(&a.doc, "s0", None)?;
    let clear_ms = timed(|| history::clear(&mut a.doc, &plan))?.1;
    let after_clear_bytes = a.doc.save().len();
    assert!(history::versions(&a.doc)?.is_empty());
    // Old history remains addressable in the causal graph after logical cleanup.
    a.doc.fork_at(&checkpoint)?;
    store.write("A", &a)?;
    let restore_ms = timed(|| restore(&mut store, "workload", &a.fingerprint()?))?.1;
    let restored = store.read("A")?;
    assert_eq!(history::versions(&restored.doc)?.len(), history_count);
    let db_before = std::fs::metadata(&path)?.len();
    let (packed_report, compaction_ms) = timed(|| maintenance::compact(&mut store))?;
    let db_after = std::fs::metadata(&path)?.len();
    Ok(
        json!({"added_cards": cards, "total_nodes": view(&a.doc)?.len(), "versions": edits,
        "description_lines": lines, "description_utf8_bytes": text(edits-1, lines).len(),
        "create_ms": create_ms, "edit": stats(&edits_ms), "sqlite_full_save": stats(&save_ms),
        "load": stats(&load_ms), "projection": stats(&projection_ms), "snapshot_ms": snapshot_ms,
        "sync_with_sqlite_ms": sync_ms, "exchange": wire, "clear_history_ms": clear_ms,
        "restore_ms": restore_ms, "compaction_ms": compaction_ms, "packing": packed_report,
        "graph_changes_before_clear": before_clear_changes, "graph_changes_after_clear": a.doc.get_changes(&[]).len(),
        "incremental_stream_bytes": incremental_bytes, "packed_bytes_before_clear": packed.len(),
        "uncompressed_columns_bytes": a.doc.save_nocompress().len(),
        "packed_bytes_after_clear": after_clear_bytes, "sqlite_before_vacuum_bytes": db_before,
        "sqlite_after_vacuum_bytes": db_after, "memory": memory()?}),
    )
}

pub(super) fn run(directory: &Path, phase: &str) -> Result<()> {
    let start = Instant::now();
    let mut data = match phase {
        "lifecycle-measure-smoke" => hierarchy(directory, 4, 10, 4)?,
        "lifecycle-measure-small" => hierarchy(directory, 32, 100, 100)?,
        "lifecycle-measure-medium" => hierarchy(directory, 256, 500, 100)?,
        "lifecycle-measure-large" => hierarchy(directory, 1024, 500, 100)?,
        "lifecycle-measure-history" => hierarchy(directory, 32, 2000, 100)?,
        "lifecycle-measure-long" => hierarchy(directory, 32, 500, 200)?,
        "lifecycle-measure-tags" => super::super::super::tag_identity::measure(directory)?,
        "lifecycle-measure-order" => super::super::super::ordering::measure(directory)?,
        _ => return Err("Unknown measurement profile".into()),
    };
    data["profile"] = phase.into();
    data["build"] = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    }
    .into();
    data["elapsed_ms"] = (start.elapsed().as_secs_f64() * 1000.0).into();
    if data.get("memory").is_none() {
        data["memory"] = memory()?;
    }
    println!("{}", serde_json::to_string(&data)?);
    Ok(())
}
